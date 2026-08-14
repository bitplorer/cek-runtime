#!/usr/bin/env node
/** Peer runtime: stdin JSON Result → stdout receipt + world. No mint. */
import { readFileSync } from "node:fs";
import { Peer } from "./peer.mjs";

if (process.argv.includes("--help") || process.argv.includes("-h")) {
  console.log("usage: node runtime.mjs [--profile ui|dom|baseline] < result.json");
  process.exit(0);
}
const i = process.argv.indexOf("--profile");
const profile = i >= 0 ? process.argv[i + 1] : "ui";
const raw = readFileSync(0, "utf8");
const result = JSON.parse(raw);
const peer = new Peer({ profile });
const receipt = peer.apply(result);
console.log(
  JSON.stringify(
    {
      receipt,
      kv: Object.fromEntries(peer.kv),
      ui: Object.fromEntries(peer.ui),
      dom: peer.dom ? peer.dom.byId() : null,
    },
    null,
    2,
  ),
);
