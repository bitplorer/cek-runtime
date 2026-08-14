#!/usr/bin/env node
/**
 * Long-lived JS Peer for the demo.
 * NDJSON in:  { "type": "apply", "result": Result }
 * NDJSON out: { "type": "applied", receipt, kv, ui, html, log }
 * No mint.
 */
import { createInterface } from "node:readline";
import { Peer } from "../../ports/cek-peer-js/peer.mjs";

const peer = new Peer({ profile: "dom" });

function snapshot() {
  return {
    kv: Object.fromEntries(peer.kv),
    ui: Object.fromEntries(peer.ui),
    log: [...peer.log],
    html: peer.dom ? peer.dom.html() : "",
    dom: peer.dom ? peer.dom.byId() : null,
  };
}

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of rl) {
  const trimmed = line.trim();
  if (!trimmed) continue;
  let msg;
  try {
    msg = JSON.parse(trimmed);
  } catch (e) {
    process.stdout.write(JSON.stringify({ type: "error", error: String(e) }) + "\n");
    continue;
  }
  if (msg.type === "apply") {
    const receipt = peer.apply(msg.result);
    process.stdout.write(
      JSON.stringify({ type: "applied", receipt, ...snapshot() }) + "\n",
    );
  } else if (msg.type === "snapshot") {
    process.stdout.write(JSON.stringify({ type: "snapshot", ...snapshot() }) + "\n");
  } else if (msg.type === "done") {
    break;
  } else {
    process.stdout.write(
      JSON.stringify({ type: "error", error: `unknown ${msg.type}` }) + "\n",
    );
  }
}
