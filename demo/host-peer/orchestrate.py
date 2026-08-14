#!/usr/bin/env python3
"""L7 demo app: Python Host decides, JS Peer applies. Contract JSON only."""

from __future__ import annotations

import html
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ports" / "cek-host-py"))
from cek_host import Host

OUT = Path(__file__).resolve().parent / "out"
SESSION = Path(__file__).resolve().parent / "peer_session.mjs"


class PeerPipe:
    def __init__(self) -> None:
        self.proc = subprocess.Popen(
            ["node", str(SESSION)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
        )

    def apply(self, result: dict) -> dict:
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(json.dumps({"type": "apply", "result": result}) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise RuntimeError("peer died")
        return json.loads(line)

    def close(self) -> None:
        if self.proc.stdin:
            self.proc.stdin.write(json.dumps({"type": "done"}) + "\n")
            self.proc.stdin.close()
        self.proc.wait(timeout=5)


def scene(trace: list, title: str, note: str, intent: dict | None, result: dict, applied: dict | None) -> None:
    trace.append(
        {
            "title": title,
            "note": note,
            "intent": intent,
            "result": result,
            "applied": applied,
        }
    )


def main() -> int:
    host = Host(now=1_000)
    peer = PeerPipe()
    trace: list[dict] = []
    try:
        # 1. Refuse — wrong action. Peer must not mutate.
        bad = {
            "action": "kv.write",
            "args": {"key": "greeting", "value": "nope"},
            "cap": {"id": "cap-bad", "action": "kv.read", "once": False},
        }
        r = host.submit(bad)
        a = peer.apply(r)
        scene(
            trace,
            "1. Refuse (action mismatch)",
            "Host refuses. ops=[]. Peer apply is a no-op. World stays empty.",
            bad,
            r,
            a,
        )
        assert r["kind"] == "authority_refusal" and r["ops"] == []
        assert a["receipt"]["landed"] == [] and a["kv"] == {}

        # 2. kv.write under an activity
        cap_kv = host.mint("cap-greet", "kv.write")
        intent_kv = {
            "action": "kv.write",
            "args": {"key": "greeting", "value": "hello"},
            "cap": cap_kv,
            "activity_id": "act-demo",
        }
        r = host.submit(intent_kv)
        a = peer.apply(r)
        scene(
            trace,
            "2. kv.write → kv.set",
            "Host projects kv.set. Peer writes greeting=hello.",
            intent_kv,
            r,
            a,
        )
        assert r["kind"] == "ok" and a["kv"].get("greeting") == "hello"

        # 3. ui.morph with snapshot (honest reverse later)
        cap_ui = host.mint("cap-ui", "ui.morph")
        snap = {
            "tag": "div",
            "attrs": {"id": "root"},
            "children": [],
        }
        patch = {
            "tag": "h1",
            "attrs": {"id": "root"},
            "children": [],
            "text": "Hello",
        }
        intent_ui = {
            "action": "ui.morph",
            "args": {"target": "root", "patch": patch, "snapshot": snap},
            "cap": cap_ui,
            "activity_id": "act-demo",
        }
        r = host.submit(intent_ui)
        a = peer.apply(r)
        scene(
            trace,
            "3. ui.morph → ui.dom.morph",
            "Host projects ui.dom.morph. Peer DomTree becomes <h1>Hello</h1>.",
            intent_ui,
            r,
            a,
        )
        assert r["kind"] == "ok"
        assert "Hello" in (a.get("html") or "")

        # 4. Receipt — landed-first reverse
        host.report_receipt("act-demo", list(a["receipt"]["landed"]))
        scene(
            trace,
            "4. Receipt",
            "Peer reports landed Ops. Host will prefer these on reverse.",
            None,
            {"kind": "receipt", "ops": [], "error": None},
            {"type": "receipt", "receipt": a["receipt"], "kv": a["kv"], "html": a.get("html")},
        )

        # 5. End activity → reverse Result → Peer apply
        rev = host.end_activity("act-demo")
        rev_result = {"kind": "ok", "ops": rev["ops"], "error": None}
        a = peer.apply(rev_result)
        scene(
            trace,
            "5. end_activity → reverse Ops",
            "Host lists inverse Ops. Peer applies them. greeting gone; DOM restored.",
            {"action": "end_activity", "args": {"activity_id": "act-demo"}, "cap": {"id": "host", "action": "end"}},
            rev_result,
            a,
        )
        assert rev["used_landed"] is True
        assert "greeting" not in a["kv"]
        assert a.get("html", "").startswith("<div")

        # 6. Once-Cap: first ok, second refuse, world unchanged
        cap_once = host.mint("cap-once", "kv.write", once=True)
        first = {
            "action": "kv.write",
            "args": {"key": "once", "value": 1},
            "cap": cap_once,
        }
        r1 = host.submit(first)
        a1 = peer.apply(r1)
        r2 = host.submit(first)
        a2 = peer.apply(r2)
        scene(
            trace,
            "6. Once-Cap",
            "First submit ok (once=1). Second refuse, ops=[]. Peer does not write again.",
            first,
            {"first": r1, "second": r2},
            {"first": a1, "second": a2},
        )
        assert r1["kind"] == "ok" and r2["kind"] == "authority_refusal" and r2["ops"] == []
        assert a2["kv"].get("once") == 1

        # 7. Expired Cap
        cap_exp = host.mint("cap-exp", "kv.write", not_after=1)
        intent_exp = {
            "action": "kv.write",
            "args": {"key": "late", "value": 1},
            "cap": cap_exp,
        }
        r = host.submit(intent_exp)
        a = peer.apply(r)
        scene(
            trace,
            "7. Expired Cap",
            "now=1000 ≥ not_after=1 → refuse. Peer never sees a late key.",
            intent_exp,
            r,
            a,
        )
        assert r["kind"] == "authority_refusal"
        assert "late" not in a["kv"]

    finally:
        peer.close()

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "trace.json").write_text(json.dumps(trace, indent=2))
    (OUT / "index.html").write_text(render_html(trace))
    print(f"demo ok — {len(trace)} scenes → {OUT / 'index.html'}")
    return 0


def render_html(trace: list[dict]) -> str:
    cards = []
    for step in trace:
        intent = json.dumps(step.get("intent"), indent=2)
        result = json.dumps(step.get("result"), indent=2)
        applied = json.dumps(step.get("applied"), indent=2)
        cards.append(
            f"""<article class="card">
  <h2>{html.escape(step["title"])}</h2>
  <p class="note">{html.escape(step["note"])}</p>
  <div class="cols">
    <section><h3>Intent + Cap</h3><pre>{html.escape(intent)}</pre></section>
    <section><h3>Result (Host → Peer)</h3><pre>{html.escape(result)}</pre></section>
    <section><h3>Peer world</h3><pre>{html.escape(applied)}</pre></section>
  </div>
</article>"""
        )
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <title>CEK demo — Python Host → JS Peer</title>
  <style>
    :root {{ color-scheme: dark; }}
    body {{ margin: 0; font: 15px/1.45 ui-sans-serif, system-ui; background: #12141a; color: #e8e6e1; }}
    header {{ padding: 2rem 1.5rem 1rem; max-width: 72rem; margin: 0 auto; }}
    h1 {{ font: 700 1.6rem/1.2 ui-serif, Georgia; margin: 0 0 .4rem; }}
    .lede {{ color: #b8b4aa; max-width: 40rem; }}
    main {{ max-width: 72rem; margin: 0 auto; padding: 0 1.5rem 3rem; display: grid; gap: 1.25rem; }}
    .card {{ background: #1b1e27; border: 1px solid #2a2e3a; border-radius: 12px; padding: 1rem 1.1rem; }}
    h2 {{ margin: 0 0 .35rem; font-size: 1.05rem; }}
    .note {{ margin: 0 0 .8rem; color: #c4bfb4; }}
    .cols {{ display: grid; gap: .7rem; }}
    @media (min-width: 900px) {{ .cols {{ grid-template-columns: 1fr 1fr 1fr; }} }}
    h3 {{ margin: 0 0 .3rem; font-size: .75rem; letter-spacing: .06em; text-transform: uppercase; color: #8b8680; }}
    pre {{ margin: 0; padding: .7rem; background: #0e1016; border-radius: 8px; overflow: auto; font: 11px/1.4 ui-monospace, monospace; color: #d5d0c7; max-height: 22rem; }}
  </style>
</head>
<body>
  <header>
    <h1>Python Host → JS Peer</h1>
    <p class="lede">One demo. Host decides (Intent → Result). Peer applies Ops. Refuse means zero Ops. Reverse is more Ops.</p>
  </header>
  <main>
    {"".join(cards)}
  </main>
</body>
</html>
"""


if __name__ == "__main__":
    raise SystemExit(main())
