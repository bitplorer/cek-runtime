#!/usr/bin/env node
/** Stress / pen for the JS Peer runtime. No mint. */
import { Peer } from "./peer.mjs";

let fails = 0;
function check(name, ok, detail = "") {
  if (ok) console.log(`PASS ${name}`);
  else {
    fails++;
    console.log(`FAIL ${name}  ${detail}`);
  }
}

const peer = new Peer({ profile: "dom" });
let landed = 0;
for (let i = 0; i < 200; i++) {
  const rec = peer.apply({
    kind: "ok",
    ops: [{ ns: "kv", name: "set", payload: { key: `k${i}`, value: i } }],
  });
  if (rec.landed.length === 1) landed++;
}
check("stress-200-kv", landed === 200, String(landed));

const refuse = peer.apply({ kind: "authority_refusal", ops: [{ ns: "kv", name: "set", payload: { key: "x", value: 1 } }] });
check("pen-refuse-no-apply", refuse.landed.length === 0 && refuse.failed.length === 0 && !peer.kv.has("x"));

const morph = peer.apply({
  kind: "ok",
  ops: [
    {
      ns: "ui.dom",
      name: "morph",
      payload: { target: "root", patch: { tag: "h1", attrs: { id: "root" }, children: [] } },
    },
  ],
});
check("dom-morph", morph.landed.length === 1 && peer.dom.get("root")?.tag === "h1");

const restore = peer.apply({
  kind: "ok",
  ops: [
    {
      ns: "ui.dom",
      name: "restore",
      payload: { target: "root", snapshot: { tag: "div", attrs: { id: "root" }, children: [] } },
    },
  ],
});
check("dom-restore", restore.landed.length === 1 && peer.dom.get("root")?.tag === "div");

peer.dom.insertChild("root", { tag: "p", attrs: { id: "blurb" }, children: [] });
peer.dom.setText("blurb", "hi");
check("dom-html", peer.dom.html().includes("<p id=\"blurb\">hi</p>"));
check("dom-path", peer.dom.get("/0")?.attrs?.id === "root");

check("no-mint", typeof peer.mint !== "function");

console.log(fails ? `${fails} failed` : "batteries ok");
process.exit(fails ? 1 : 0);
