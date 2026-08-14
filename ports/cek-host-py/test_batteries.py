#!/usr/bin/env python3
"""Stress / load / chaos / pen for the Python Host runtime."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from cek_host import Host, attach_hmac

fails = 0


def check(name: str, ok: bool, detail: str = "") -> None:
    global fails
    if ok:
        print(f"PASS {name}")
    else:
        fails += 1
        print(f"FAIL {name}  {detail}")


def intent(cap, key="k", value=1, **kw):
    i = {
        "action": "kv.write",
        "args": {"key": key, "value": value},
        "cap": cap,
    }
    i.update(kw)
    return i


def refuse(r) -> bool:
    return r["kind"] == "authority_refusal" and r.get("ops") == []


def main() -> int:
    h = Host(now=1000)
    for i in range(300):
        r = h.submit(intent(h.mint(f"c{i}", "kv.write"), key=f"k{i}", value=i))
        if r["kind"] != "ok":
            check("stress-300", False, str(r))
            break
    else:
        check("stress-300", True)

    h = Host(now=1000)
    cap = h.mint("once", "kv.write", once=True)
    ok = refuse_n = 0
    for _ in range(20):
        r = h.submit(intent(cap, key="k"))
        if r["kind"] == "ok":
            ok += 1
        elif refuse(r):
            refuse_n += 1
    check("stress-once-one-ok", ok == 1 and refuse_n == 19, f"ok={ok} refuse={refuse_n}")

    h = Host(now=1000)
    leaked = False
    for bad in (
        intent({"id": "x", "action": "kv.read", "once": False}),
        intent(h.mint("s", "kv.write") | {"subject": "a"}, **{"args": {"key": "k", "value": 1, "subject": "b"}}),
    ):
        # rebuild spoof cleanly
        pass
    cap = h.mint("mm", "kv.read")
    check("pen-mismatch", refuse(h.submit(intent(cap))))

    key = bytes(32)
    h = Host(now=1000, hmac_key=key)
    cap = attach_hmac(h.mint("h", "kv.write"), key)
    cap["sig"] = cap["sig"][:-1] + ("0" if cap["sig"][-1] != "0" else "1")
    check("pen-hmac-flip", refuse(h.submit(intent(cap))))

    h = Host(now=1000, hmac_key=key)
    cap = h.mint("u", "kv.write")
    cap.pop("sig", None)
    check("pen-hmac-unsigned", refuse(h.submit(intent(cap))))

    h = Host(now=1000)
    cap = h.mint("g", "kv.write")
    cap["law_generation"] = "not-a-law"
    check("pen-law-gen", refuse(h.submit(intent(cap))))

    from ed25519 import public_key, sign, verify

    seed = bytes.fromhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    pk = public_key(seed)
    sig = sign(seed, b"")
    check(
        "ed25519-rfc8032",
        pk.hex() == "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        and verify(pk, b"", sig),
    )

    print("batteries", "ok" if fails == 0 else f"{fails} failed")
    return 1 if fails else 0


if __name__ == "__main__":
    raise SystemExit(main())
