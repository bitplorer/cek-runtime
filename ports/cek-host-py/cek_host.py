"""CEK Host runtime (Python) — decide role.

Contains a Host kernel: verify Cap, project Ops, once, lineage reverse.
Not law. Peer ports must not import mint.
"""

from __future__ import annotations

import hashlib
import hmac
import json
from dataclasses import dataclass, field
from typing import Any

LAW = "cek-law-1"
KIND_OK = "ok"
KIND_REFUSE = "authority_refusal"
KIND_DISPATCH = "dispatch_error"


def _canon(obj: Any) -> bytes:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def cek1_hex(data: bytes) -> str:
    return "cek1:" + hashlib.sha256(data).hexdigest()


def cap_sign_bytes(cap: dict) -> bytes:
    return _canon(
        {
            "action": cap.get("action"),
            "id": cap.get("id"),
            "law_generation": cap.get("law_generation"),
            "not_after": cap.get("not_after"),
            "once": bool(cap.get("once", False)),
            "scopes": cap.get("scopes") or [],
            "sealed_args_bind": cap.get("sealed_args_bind"),
            "subject": cap.get("subject"),
        }
    )


def attach_hmac(cap: dict, key: bytes) -> dict:
    out = dict(cap)
    out["sig"] = "cek1:" + hmac.new(key, cap_sign_bytes(out), hashlib.sha256).hexdigest()
    return out


def hmac_valid(cap: dict, key: bytes) -> bool:
    sig = cap.get("sig")
    if not isinstance(sig, str):
        return False
    expect = "cek1:" + hmac.new(key, cap_sign_bytes(cap), hashlib.sha256).hexdigest()
    return hmac.compare_digest(sig, expect)


def resource_of(intent: dict) -> tuple[str, str]:
    action = intent.get("action") or ""
    args = intent.get("args") or {}
    if action in ("kv.write", "kv.delete"):
        return "kv", str(args.get("key") or "")
    if action in ("ui.morph", "ui.restore"):
        return "ui", str(args.get("target") or "")
    if action == "log.append":
        return "log", ""
    return "action", action


def scope_allows(scope: str, kind: str, name: str) -> bool:
    scope = scope.strip()
    if not scope:
        return False
    if scope == kind or (name and scope == name):
        return True
    if ":" in scope:
        k, n = scope.split(":", 1)
        return k == kind and (n == "*" or (n and n == name))
    return False


def inverse_op(op: dict) -> dict | None:
    ns, name, p = op.get("ns"), op.get("name"), op.get("payload") or {}
    if ns == "kv" and name == "set":
        return {"ns": "kv", "name": "delete", "payload": {"key": p.get("key")}}
    if ns == "kv" and name == "delete" and "prior" in p and p["prior"] is not None:
        return {"ns": "kv", "name": "set", "payload": {"key": p.get("key"), "value": p["prior"]}}
    if ns == "ui.dom" and name == "morph" and "snapshot" in p:
        return {
            "ns": "ui.dom",
            "name": "restore",
            "payload": {"target": p.get("target"), "snapshot": p["snapshot"]},
        }
    return None


def project(intent: dict) -> list[dict]:
    action = intent.get("action") or ""
    args = intent.get("args") or {}
    if action == "kv.write":
        key = args.get("key")
        if not isinstance(key, str) or not key:
            raise Dispatch("kv.write key must be non-empty")
        return [{"ns": "kv", "name": "set", "payload": {"key": key, "value": args.get("value")}}]
    if action == "kv.delete":
        key = args.get("key")
        if not isinstance(key, str) or not key:
            raise Dispatch("kv.delete key must be non-empty")
        payload: dict[str, Any] = {"key": key}
        if "prior" in args:
            payload["prior"] = args["prior"]
        return [{"ns": "kv", "name": "delete", "payload": payload}]
    if action == "log.append":
        msg = args.get("message")
        if not isinstance(msg, str):
            raise Dispatch("log.append requires string args.message")
        return [{"ns": "log", "name": "append", "payload": {"message": msg}}]
    if action == "ui.morph":
        target = args.get("target")
        if not isinstance(target, str) or not target:
            raise Dispatch("ui.morph target must be non-empty")
        if "patch" not in args:
            raise Dispatch("ui.morph requires args.patch")
        payload = {"target": target, "patch": args["patch"]}
        if "snapshot" in args:
            payload["snapshot"] = args["snapshot"]
        return [{"ns": "ui.dom", "name": "morph", "payload": payload}]
    if action == "ui.restore":
        target = args.get("target")
        if not isinstance(target, str) or not target:
            raise Dispatch("ui.restore target must be non-empty")
        if "snapshot" not in args:
            raise Dispatch("ui.restore requires args.snapshot")
        return [
            {
                "ns": "ui.dom",
                "name": "restore",
                "payload": {"target": target, "snapshot": args["snapshot"]},
            }
        ]
    raise Dispatch(f"unknown action: {action}")


class Authority(Exception):
    pass


class Dispatch(Exception):
    pass


@dataclass
class LineageEntry:
    ops: list[dict]
    inverse: list[dict]
    reverse_class: str
    landed: list[dict] = field(default_factory=list)
    ended: bool = False


class Host:
    """In-process Host runtime (kernel + memory stores)."""

    def __init__(
        self,
        now: int = 0,
        hmac_key: bytes | None = None,
        accepted: list[str] | None = None,
    ):
        self.now = now
        self.hmac_key = hmac_key
        self.accepted = list(accepted or [LAW])
        if LAW not in self.accepted:
            self.accepted.insert(0, LAW)
        self.once_used: set[str] = set()
        self.idem: dict[str, dict] = {}
        self.lineage: dict[str, list[LineageEntry]] = {}
        self.ended: set[str] = set()

    def mint(self, id: str, action: str, once: bool = False, not_after=None) -> dict:
        cap = {
            "id": id,
            "action": action,
            "once": once,
            "not_after": not_after,
            "scopes": [],
            "law_generation": LAW,
        }
        if self.hmac_key:
            cap = attach_hmac(cap, self.hmac_key)
        return cap

    def submit(self, intent: dict) -> dict:
        try:
            self._verify(intent)
        except Authority as e:
            return {"kind": KIND_REFUSE, "ops": [], "error": str(e)}
        key = intent.get("idempotency_key")
        if isinstance(key, str) and not key.strip():
            return {"kind": KIND_REFUSE, "ops": [], "error": "empty idempotency key"}
        try:
            ops = project(intent)
        except Dispatch as e:
            return {"kind": KIND_DISPATCH, "ops": [], "error": str(e)}
        body = json.dumps(ops, sort_keys=True, separators=(",", ":"))
        if isinstance(key, str):
            if key in self.idem:
                prior = self.idem[key]
                if json.dumps(prior.get("ops"), sort_keys=True, separators=(",", ":")) != body:
                    return {"kind": KIND_REFUSE, "ops": [], "error": "idempotency conflict"}
                return dict(prior)
        aid = intent.get("activity_id")
        if aid == "":
            return {"kind": KIND_DISPATCH, "ops": [], "error": "empty activity_id"}
        cap = intent.get("cap") or {}
        if cap.get("once"):
            cid = cap.get("id") or ""
            if cid in self.once_used:
                return {"kind": KIND_REFUSE, "ops": [], "error": "once Cap already used"}
            self.once_used.add(cid)
        result = {"kind": KIND_OK, "ops": ops, "error": None}
        if isinstance(key, str):
            self.idem[key] = result
        if isinstance(aid, str) and aid.strip():
            if aid in self.ended:
                return {"kind": KIND_DISPATCH, "ops": [], "error": "activity already ended"}
            inv = [x for o in reversed(ops) if (x := inverse_op(o))]
            cls = "Inverse" if inv else "NonReversible"
            self.lineage.setdefault(aid, []).append(LineageEntry(ops, inv, cls))
        return result

    def report_receipt(self, activity_id: str, landed: list[dict]) -> None:
        ents = self.lineage.get(activity_id) or []
        if not ents:
            raise Dispatch("unknown activity")
        ents[-1].landed = list(landed)

    def end_activity(self, activity_id: str) -> dict:
        if not activity_id:
            raise Dispatch("empty activity_id")
        if activity_id in self.ended:
            raise Dispatch("already ended")
        ents = self.lineage.get(activity_id) or []
        self.ended.add(activity_id)
        ops: list[dict] = []
        non: list[int] = []
        used_landed = False
        for i, e in enumerate(reversed(ents)):
            if e.reverse_class != "Inverse":
                non.append(i)
                continue
            if e.landed:
                used_landed = True
                ops.extend(x for o in reversed(e.landed) if (x := inverse_op(o)))
            else:
                ops.extend(e.inverse)
        return {"ops": ops, "non_reversible": non, "used_landed": used_landed}

    def _verify(self, intent: dict) -> None:
        cap = intent.get("cap") or {}
        if (intent.get("action") or "") != (cap.get("action") or ""):
            raise Authority("action mismatch")
        if not (intent.get("action") or "").strip() or not (cap.get("action") or "").strip():
            raise Authority("empty action is not allowed")
        if not (cap.get("id") or "").strip():
            raise Authority("empty Cap id is not allowed")
        na = cap.get("not_after")
        if na is not None and self.now >= int(na):
            raise Authority("Cap expired")
        gen = cap.get("law_generation")
        if isinstance(gen, str):
            if not gen.strip():
                raise Authority("empty law generation is not allowed")
            if gen not in self.accepted:
                raise Authority(f"law generation `{gen}` not accepted")
        bind = cap.get("sealed_args_bind")
        if bind:
            got = cek1_hex(_canon(intent.get("args") or {}))
            if got != bind:
                raise Authority("sealed-args bind mismatch")
        scopes = cap.get("scopes") or []
        if any(not str(s).strip() for s in scopes):
            raise Authority("empty scope token is not allowed")
        if scopes:
            kind, name = resource_of(intent)
            if not any(scope_allows(str(s), kind, name) for s in scopes):
                raise Authority("scope does not allow resource")
        subj = cap.get("subject")
        if isinstance(subj, str):
            if not subj.strip():
                raise Authority("empty Cap subject is not allowed")
            got = (intent.get("args") or {}).get("subject")
            if got != subj:
                raise Authority("subject bind mismatch")
        if self.hmac_key is not None:
            if not hmac_valid(cap, self.hmac_key):
                raise Authority("Cap signature required or invalid")
