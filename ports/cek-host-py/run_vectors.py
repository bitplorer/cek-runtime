#!/usr/bin/env python3
"""Run CORE vectors against the Python Host runtime. Skip Peer-only."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from cek_host import Host, attach_hmac, attach_ed25519, Dispatch

ROOT = Path(__file__).resolve().parents[2]


def load_cases(d: Path) -> list[dict]:
    out = []
    for p in sorted(d.glob("*.json")):
        out.append(json.loads(p.read_text()))
    return out


def hexkey(s: str) -> bytes:
    return bytes.fromhex(s.strip())


def run_case(c: dict) -> str | None:
    if c.get("peer_result") and not c.get("intent"):
        return "skip-peer-only"
    now = int(c.get("now") or 0)
    key = hexkey(c["hmac_key"]) if c.get("hmac_key") else None
    ed = hexkey(c["ed25519_seed"]) if c.get("ed25519_seed") else None
    accepted = [*(c.get("accept_generations") or [])]
    host = Host(now=now, hmac_key=key, ed25519_seed=ed, accepted=accepted or None)
    if c.get("prior_intent"):
        pr = host.submit(c["prior_intent"])
        if c.get("prior_must_ok") and pr["kind"] != "ok":
            return f"prior not ok: {pr}"
    if c.get("prior_end_activity"):
        host.end_activity(c["prior_end_activity"])
    intent = c.get("intent")
    if not intent:
        return "skip-no-intent"
    if c.get("sign_cap"):
        intent = dict(intent)
        cap = dict(intent.get("cap") or {})
        if ed:
            intent["cap"] = attach_ed25519(cap, ed)
        elif key:
            intent["cap"] = attach_hmac(cap, key)
    r = host.submit(intent)
    if r["kind"] != c["expect_kind"]:
        return f"kind {r['kind']} != {c['expect_kind']} ({r.get('error')})"
    if c.get("expect_ops_empty") and r.get("ops"):
        return f"ops not empty: {r['ops']}"
    if c.get("expect_ops") is not None:
        if r.get("ops") != c["expect_ops"]:
            return f"ops {r.get('ops')} != {c['expect_ops']}"
    if c.get("report_receipt") and r.get("kind") == "ok":
        aid = c.get("end_activity") or (intent.get("activity_id") if intent else None)
        if aid:
            host.report_receipt(aid, list(r.get("ops") or []))
    if c.get("end_activity"):
        try:
            rev = host.end_activity(c["end_activity"])
        except Dispatch as e:
            return f"end: {e}"
        if c.get("expect_reverse_ops") is not None and rev["ops"] != c["expect_reverse_ops"]:
            return f"reverse {rev['ops']} != {c['expect_reverse_ops']}"
        if c.get("expect_used_landed") is not None and rev["used_landed"] != c["expect_used_landed"]:
            return f"used_landed {rev['used_landed']}"
    if c.get("end_activity_again"):
        try:
            host.end_activity(c["end_activity"])
            return "second end should fail"
        except Dispatch:
            pass
    if c.get("expect_once_consumed") is True:
        cid = (intent.get("cap") or {}).get("id")
        if cid not in host.once_used:
            return "once not consumed"
    return None


def main() -> int:
    d = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "crates/cek-contract/vectors")
    passed = failed = skipped = 0
    for c in load_cases(d):
        err = run_case(c)
        if err and err.startswith("skip"):
            skipped += 1
            continue
        if err:
            failed += 1
            print(f"FAIL {c['id']}  {err}")
        else:
            passed += 1
            print(f"PASS {c['id']}")
    print(f"\n{passed} passed, {failed} failed, {skipped} skipped (python Host)")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
