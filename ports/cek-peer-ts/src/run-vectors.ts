/**
 * Apply-side vector runner: same JSON fixtures as Rust.
 * Only cases with peer_apply or peer_result are executed (Peer has no mint).
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { Peer, type ResultMsg } from "./peer.ts";

type Case = {
  id: string;
  family: string;
  peer_apply?: boolean;
  peer_result?: ResultMsg;
  peer_profile?: string;
  peer_unknown_policy?: string;
  expect_peer_kv?: Record<string, unknown>;
  expect_peer_ui?: Record<string, unknown>;
};

function loadDir(dir: string): Case[] {
  return readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .sort()
    .map((f) => JSON.parse(readFileSync(join(dir, f), "utf8")) as Case);
}

function checkMap(
  label: string,
  got: Map<string, unknown>,
  expect: Record<string, unknown>,
): string | null {
  for (const [k, v] of Object.entries(expect)) {
    const have = got.has(k) ? got.get(k) : undefined;
    if (v === null) {
      if (have !== undefined) return `${label}[${k}] should be absent, got ${JSON.stringify(have)}`;
    } else if (JSON.stringify(have) !== JSON.stringify(v)) {
      return `${label}[${k}] want ${JSON.stringify(v)} got ${JSON.stringify(have)}`;
    }
  }
  return null;
}

function main() {
  const dir = process.argv[2] ?? "crates/cek-contract/vectors";
  const all = loadDir(dir);
  const cases = all.filter((c) => c.peer_apply || c.peer_result);
  const hostOnly = all.length - cases.length;
  let passed = 0;
  let failed = 0;
  let skippedHost = 0;
  for (const c of cases) {
    if (!c.peer_result) {
      // Host-projected apply cases need the Result from Host; this port
      // only runs Peer-only fixtures (peer_result set).
      skippedHost++;
      continue;
    }
    const peer = new Peer({
      profile: c.peer_profile === "ui" ? "ui" : "baseline",
      unknownOpPolicy: c.peer_unknown_policy === "fail_batch" ? "fail_batch" : "skip",
    });
    peer.apply(c.peer_result);
    const err =
      (c.expect_peer_kv && checkMap("kv", peer.kv, c.expect_peer_kv)) ||
      (c.expect_peer_ui && checkMap("ui", peer.ui, c.expect_peer_ui));
    if (err) {
      console.error(`FAIL ${c.id}: ${err}`);
      failed++;
    } else {
      console.log(`PASS ${c.id}  [${c.family}]`);
      passed++;
    }
  }
  // Always run the built-in apply self-check so the port is never empty.
  const self = new Peer({ profile: "ui" });
  const rec = self.apply({
    kind: "ok",
    ops: [
      { ns: "kv", name: "set", payload: { key: "a", value: 1 } },
      {
        ns: "ui.dom",
        name: "morph",
        payload: { target: "hdr", patch: { t: "n" }, snapshot: { t: "o" } },
      },
    ],
  });
  const refuse = new Peer().apply({ kind: "authority_refusal", ops: [], error: "no" });
  const okSelf =
    rec.landed.length === 2 &&
    self.kv.get("a") === 1 &&
    JSON.stringify(self.ui.get("hdr")) === JSON.stringify({ t: "n" }) &&
    refuse.landed.length === 0;
  if (!okSelf) {
    console.error("FAIL ts-peer-self");
    failed++;
  } else {
    console.log("PASS ts-peer-self  [port]");
    passed++;
  }
  console.log(
    `\n${passed} passed, ${failed} failed (ts peer apply-only; skipped ${skippedHost + hostOnly} host-projected)`,
  );
  if (failed > 0) process.exit(1);
}

main();
