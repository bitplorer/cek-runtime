#!/usr/bin/env python3
"""Apply-only PyO3 Peer runner — no mint.

Same peer_result fixtures as ports/cek-peer-wasm.
Public surface is apply/receipt only (construct/bind/release around it).
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path


def load_mod(so_path: str):
    spec = importlib.util.spec_from_file_location("cek_peer_pyo3", so_path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load {so_path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def load_dir(d: str):
    p = Path(d)
    return [
        json.loads(f.read_text())
        for f in sorted(p.iterdir())
        if f.suffix == ".json"
    ]


def check_map(label, got, expect):
    for k, v in expect.items():
        have = got[k] if k in got else None
        if v is None:
            if have is not None:
                return f"{label}[{k}] should be absent, got {json.dumps(have)}"
        elif json.dumps(have, sort_keys=True) != json.dumps(v, sort_keys=True):
            return f"{label}[{k}] want {json.dumps(v)} got {json.dumps(have)}"
    return None


def apply_session(mod, req):
    abi = mod.PeerAbi.construct()
    abi.bind()
    try:
        return abi.apply(req)
    finally:
        abi.release()


def find_cek():
    env = os.environ.get("CEK_BIN")
    if env and Path(env).is_file():
        return env
    root = Path(__file__).resolve().parents[2]
    for rel in ("target/release/cek", "target/debug/cek"):
        cand = root / rel
        if cand.is_file():
            return str(cand)
    return None


def cli_apply(cek_bin, req):
    p = subprocess.run(
        [cek_bin, "apply"],
        input=json.dumps(req),
        text=True,
        capture_output=True,
        check=False,
    )
    if p.returncode != 0:
        raise RuntimeError(f"cek apply failed: {p.stderr}")
    return json.loads(p.stdout)


def main():
    dir_ = sys.argv[1] if len(sys.argv) > 1 else "crates/cek-contract/vectors"
    so = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("CEK_PEER_PYO3")
    if not so:
        root = Path(__file__).resolve().parents[2]
        for rel in (
            "target/release/libcek_peer_pyo3.so",
            "target/debug/libcek_peer_pyo3.so",
            "target/release/cek_peer_pyo3.so",
        ):
            cand = root / rel
            if cand.is_file():
                so = str(cand)
                break
    if not so:
        print("usage: run-vectors.py [vectors-dir] [libcek_peer_pyo3.so]", file=sys.stderr)
        sys.exit(2)

    mod = load_mod(so)
    names = {n for n in dir(mod) if not n.startswith("_")}
    if "PeerAbi" not in names:
        print("FAIL: module missing PeerAbi", file=sys.stderr)
        sys.exit(1)
    extra = names - {"PeerAbi"}
    if extra:
        print(f"FAIL: public names besides PeerAbi: {sorted(extra)}", file=sys.stderr)
        sys.exit(1)
    surface = {
        n
        for n in dir(mod.PeerAbi)
        if not n.startswith("_") and n not in {"mro"}
    }
    # PyO3 may expose constructor aliases; apply/receipt lifecycle only.
    allowed = {"construct", "bind", "apply", "release"}
    banned = {n for n in surface if any(b in n.lower() for b in ("mint", "verify", "host", "submit"))}
    if banned:
        print(f"FAIL: mint/host verbs on surface: {sorted(banned)}", file=sys.stderr)
        sys.exit(1)
    missing = allowed - surface
    if missing:
        print(f"FAIL: missing lifecycle/apply: {sorted(missing)}", file=sys.stderr)
        sys.exit(1)

    cek_bin = find_cek()
    passed = 0
    failed = 0
    skipped = 0
    for c in load_dir(dir_):
        if not c.get("peer_result"):
            skipped += 1
            continue
        req = {
            "result": c["peer_result"],
            "profile": "ui" if c.get("peer_profile") == "ui" else "baseline",
            "unknown_op_policy": (
                "fail_batch" if c.get("peer_unknown_policy") == "fail_batch" else "skip"
            ),
        }
        resp = apply_session(mod, req)
        err = (c.get("expect_peer_kv") and check_map("kv", resp.get("kv") or {}, c["expect_peer_kv"])) or (
            c.get("expect_peer_ui") and check_map("ui", resp.get("ui") or {}, c["expect_peer_ui"])
        )
        if err:
            print(f"FAIL {c['id']}: {err}", file=sys.stderr)
            failed += 1
            continue
        if cek_bin:
            cli = cli_apply(cek_bin, req)
            if json.dumps(resp, sort_keys=True) != json.dumps(cli, sort_keys=True):
                print(f"FAIL {c['id']}: pyo3 != cek apply", file=sys.stderr)
                failed += 1
                continue
        print(f"PASS {c['id']}  [{c.get('family')}]")
        passed += 1

    self_req = {
        "result": {
            "kind": "ok",
            "ops": [
                {"ns": "kv", "name": "set", "payload": {"key": "a", "value": 1}},
                {"ns": "log", "name": "append", "payload": {"message": "hello"}},
                {
                    "ns": "ui.dom",
                    "name": "morph",
                    "payload": {"target": "hdr", "patch": {"t": "n"}, "snapshot": {"t": "o"}},
                },
            ],
        },
        "profile": "ui",
    }
    self = apply_session(mod, self_req)
    ok = (
        self.get("kv", {}).get("a") == 1
        and self.get("log") == ["hello"]
        and self.get("ui", {}).get("hdr") == {"t": "n"}
        and len(self.get("receipt", {}).get("landed") or []) == 3
    )
    if ok and cek_bin:
        ok = json.dumps(self, sort_keys=True) == json.dumps(cli_apply(cek_bin, self_req), sort_keys=True)
    if ok:
        print("PASS pyo3-peer-self  [port]")
        passed += 1
    else:
        print("FAIL pyo3-peer-self", file=sys.stderr)
        failed += 1

    cli_note = " + cek apply" if cek_bin else " (no cek binary; rust tests cover apply_json path)"
    print(
        f"\n{passed} passed, {failed} failed "
        f"(pyo3 peer apply-only{cli_note}; skipped {skipped} host-projected)"
    )
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
