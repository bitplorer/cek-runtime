/**
 * CEK Peer — TypeScript apply-only port.
 *
 * No mint. No Cap verify. Apply Ops, return a receipt.
 * Same unknown-Op policy and Baseline + ui.dom.* drivers as the Rust Peer.
 */

export type Op = { ns: string; name: string; payload: Record<string, unknown> | unknown };
export type ResultKind = "ok" | "authority_refusal" | "dispatch_error";
export type ResultMsg = {
  kind: ResultKind;
  ops?: Op[];
  error?: string | null;
  digest?: string | null;
};
export type Receipt = { landed: Op[]; failed: Op[] };
export type UnknownOpPolicy = "skip" | "fail_batch";

export function fq(op: Op): string {
  return `${op.ns}.${op.name}`;
}

const BASELINE = ["kv.set", "kv.delete", "log.append"];
const UI_OPS = ["ui.dom.morph", "ui.dom.restore"];

export class Peer {
  readonly profileName: string;
  readonly applySet: Set<string>;
  readonly unknownOpPolicy: UnknownOpPolicy;
  readonly kv = new Map<string, unknown>();
  readonly log: string[] = [];
  readonly ui = new Map<string, unknown>();

  constructor(opts?: {
    profile?: "baseline" | "ui";
    unknownOpPolicy?: UnknownOpPolicy;
  }) {
    const profile = opts?.profile ?? "baseline";
    this.profileName = profile;
    this.unknownOpPolicy = opts?.unknownOpPolicy ?? "skip";
    const set = [...BASELINE];
    if (profile === "ui") set.push(...UI_OPS);
    this.applySet = new Set(set);
  }

  apply(result: ResultMsg): Receipt {
    if (result.kind === "authority_refusal" || result.kind === "dispatch_error") {
      return { landed: [], failed: [] };
    }
    const landed: Op[] = [];
    const failed: Op[] = [];
    let abort = false;
    for (const op of result.ops ?? []) {
      if (abort) {
        failed.push(op);
        continue;
      }
      if (!this.applySet.has(fq(op))) {
        failed.push(op);
        if (this.unknownOpPolicy === "fail_batch") abort = true;
        continue;
      }
      if (this.applyOne(op)) landed.push(op);
      else failed.push(op);
    }
    return { landed, failed };
  }

  private applyOne(op: Op): boolean {
    const p = (op.payload ?? {}) as Record<string, unknown>;
    if (op.ns === "kv" && op.name === "set") {
      if (typeof p.key !== "string") return false;
      this.kv.set(p.key, p.value ?? null);
      return true;
    }
    if (op.ns === "kv" && op.name === "delete") {
      if (typeof p.key !== "string") return false;
      this.kv.delete(p.key);
      return true;
    }
    if (op.ns === "log" && op.name === "append") {
      if (typeof p.message !== "string") return false;
      this.log.push(p.message);
      return true;
    }
    if (op.ns === "ui.dom" && op.name === "morph") {
      if (typeof p.target !== "string" || !("patch" in p)) return false;
      this.ui.set(p.target, p.patch);
      return true;
    }
    if (op.ns === "ui.dom" && op.name === "restore") {
      if (typeof p.target !== "string" || !("snapshot" in p)) return false;
      this.ui.set(p.target, p.snapshot);
      return true;
    }
    return false;
  }
}
