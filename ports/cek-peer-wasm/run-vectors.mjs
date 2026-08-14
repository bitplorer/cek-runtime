#!/usr/bin/env node
/**
 * Apply-only WASM Peer runner — no mint.
 * Same peer_result fixtures as ports/cek-peer-ts.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const wasmPath = process.argv[3];
const dir = process.argv[2] ?? "crates/cek-contract/vectors";

function loadDir(d) {
  return readdirSync(d)
    .filter((f) => f.endsWith(".json"))
    .sort()
    .map((f) => JSON.parse(readFileSync(join(d, f), "utf8")));
}

function checkMap(label, got, expect) {
  for (const [k, v] of Object.entries(expect)) {
    const have = Object.prototype.hasOwnProperty.call(got, k) ? got[k] : undefined;
    if (v === null) {
      if (have !== undefined) {
        return `${label}[${k}] should be absent, got ${JSON.stringify(have)}`;
      }
    } else if (JSON.stringify(have) !== JSON.stringify(v)) {
      return `${label}[${k}] want ${JSON.stringify(v)} got ${JSON.stringify(have)}`;
    }
  }
  return null;
}

const bytes = readFileSync(wasmPath);
const { instance } = await WebAssembly.instantiate(bytes, {});
const { memory, cek_alloc, cek_apply, cek_result_ptr } = instance.exports;
if (typeof cek_apply !== "function") {
  console.error("WASM missing cek_apply (not an apply-only Peer module)");
  process.exit(1);
}

function apply(req) {
  const enc = new TextEncoder().encode(JSON.stringify(req));
  const p = cek_alloc(enc.length);
  new Uint8Array(memory.buffer, Number(p), enc.length).set(enc);
  const n = cek_apply(p, enc.length);
  if (n < 0) throw new Error("cek_apply failed");
  const rp = Number(cek_result_ptr());
  const out = new Uint8Array(memory.buffer, rp, Number(n));
  return JSON.parse(new TextDecoder().decode(out));
}

const all = loadDir(dir);
let passed = 0;
let failed = 0;
let skipped = 0;
for (const c of all) {
  if (!c.peer_result) {
    skipped++;
    continue;
  }
  const resp = apply({
    result: c.peer_result,
    profile: c.peer_profile === "ui" ? "ui" : "baseline",
    unknown_op_policy: c.peer_unknown_policy === "fail_batch" ? "fail_batch" : "skip",
  });
  const err =
    (c.expect_peer_kv && checkMap("kv", resp.kv ?? {}, c.expect_peer_kv)) ||
    (c.expect_peer_ui && checkMap("ui", resp.ui ?? {}, c.expect_peer_ui));
  if (err) {
    console.error(`FAIL ${c.id}: ${err}`);
    failed++;
  } else {
    console.log(`PASS ${c.id}  [${c.family}]`);
    passed++;
  }
}

const self = apply({
  result: {
    kind: "ok",
    ops: [
      { ns: "kv", name: "set", payload: { key: "a", value: 1 } },
      {
        ns: "ui.dom",
        name: "morph",
        payload: { target: "hdr", patch: { t: "n" }, snapshot: { t: "o" } },
      },
    ],
  },
  profile: "ui",
});
if (self.kv?.a === 1 && JSON.stringify(self.ui?.hdr) === JSON.stringify({ t: "n" })) {
  console.log("PASS wasm-peer-self  [port]");
  passed++;
} else {
  console.error("FAIL wasm-peer-self");
  failed++;
}

console.log(
  `\n${passed} passed, ${failed} failed (wasm peer apply-only; skipped ${skipped} host-projected)`,
);
process.exit(failed > 0 ? 1 : 0);
