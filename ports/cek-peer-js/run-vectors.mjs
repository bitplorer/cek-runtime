#!/usr/bin/env node
/** Apply-only vector runner. No mint. */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { Peer } from "./peer.mjs";

const dir = process.argv[2] ?? "crates/cek-contract/vectors";
const files = readdirSync(dir).filter((f) => f.endsWith(".json")).sort();
let passed = 0;
let failed = 0;
let skipped = 0;

for (const f of files) {
  const c = JSON.parse(readFileSync(join(dir, f), "utf8"));
  if (!c.peer_result) {
    skipped++;
    continue;
  }
  const peer = new Peer({
    profile: c.peer_profile === "ui" || c.peer_profile === "dom" ? c.peer_profile : "baseline",
    unknownOpPolicy: c.peer_unknown_policy === "fail_batch" ? "fail_batch" : "skip",
  });
  const rec = peer.apply(c.peer_result);
  let err = null;
  if (c.expect_peer_kv) {
    for (const [k, v] of Object.entries(c.expect_peer_kv)) {
      const have = peer.kv.has(k) ? peer.kv.get(k) : undefined;
      if (v === null && have !== undefined) err = `kv[${k}] should be absent`;
      else if (v !== null && JSON.stringify(have) !== JSON.stringify(v)) {
        err = `kv[${k}] want ${JSON.stringify(v)} got ${JSON.stringify(have)}`;
      }
    }
  }
  if (!err && c.expect_peer_ui) {
    for (const [k, v] of Object.entries(c.expect_peer_ui)) {
      const have = peer.ui.has(k) ? peer.ui.get(k) : undefined;
      if (v === null && have !== undefined) err = `ui[${k}] should be absent`;
      else if (v !== null && JSON.stringify(have) !== JSON.stringify(v)) {
        err = `ui[${k}] want ${JSON.stringify(v)} got ${JSON.stringify(have)}`;
      }
    }
  }
  if (err) {
    failed++;
    console.log(`FAIL ${c.id}  ${err}`);
  } else {
    passed++;
    console.log(`PASS ${c.id}  landed=${rec.landed.length}`);
  }
}

console.log(`\n${passed} passed, ${failed} failed, ${skipped} skipped (js Peer apply-only)`);
process.exit(failed ? 1 : 0);
