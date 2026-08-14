/**
 * CEK Peer runtime (JavaScript) — apply only.
 * Drivers: kv, log, DomTree. No mint.
 */

export function fq(op) {
  return `${op.ns}.${op.name}`;
}

const BASELINE = ["kv.set", "kv.delete", "log.append"];
const UI_OPS = ["ui.dom.morph", "ui.dom.restore"];

export class DomTree {
  constructor() {
    this.roots = [
      { tag: "div", attrs: { id: "root" }, children: [] },
    ];
  }

  morph(target, patch) {
    return walkReplace(this.roots, target, patch);
  }

  restore(target, snapshot) {
    return this.morph(target, snapshot);
  }

  get(target) {
    return findId(this.roots, target);
  }

  byId() {
    const out = {};
    collect(this.roots, out);
    return out;
  }
}

function idOf(n) {
  return n?.attrs?.id;
}

function walkReplace(nodes, target, patch) {
  for (let i = 0; i < nodes.length; i++) {
    if (idOf(nodes[i]) === target) {
      nodes[i] = patch;
      return true;
    }
    if (Array.isArray(nodes[i].children) && walkReplace(nodes[i].children, target, patch)) {
      return true;
    }
  }
  return false;
}

function findId(nodes, target) {
  for (const n of nodes) {
    if (idOf(n) === target) return n;
    if (Array.isArray(n.children)) {
      const hit = findId(n.children, target);
      if (hit) return hit;
    }
  }
  return undefined;
}

function collect(nodes, out) {
  for (const n of nodes) {
    const id = idOf(n);
    if (id) out[id] = n;
    if (Array.isArray(n.children)) collect(n.children, out);
  }
}

export class Peer {
  constructor(opts = {}) {
    const profile = opts.profile ?? "baseline";
    this.profileName = profile;
    this.unknownOpPolicy = opts.unknownOpPolicy ?? "skip";
    const set = [...BASELINE];
    if (profile === "ui" || profile === "dom") set.push(...UI_OPS);
    this.applySet = new Set(set);
    this.kv = new Map();
    this.log = [];
    this.ui = new Map();
    this.dom = profile === "dom" ? new DomTree() : null;
  }

  apply(result) {
    if (result.kind === "authority_refusal" || result.kind === "dispatch_error") {
      return { landed: [], failed: [] };
    }
    const landed = [];
    const failed = [];
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

  applyOne(op) {
    const p = op.payload ?? {};
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
      if (this.dom) this.dom.morph(p.target, p.patch);
      return true;
    }
    if (op.ns === "ui.dom" && op.name === "restore") {
      if (typeof p.target !== "string" || !("snapshot" in p)) return false;
      this.ui.set(p.target, p.snapshot);
      if (this.dom) this.dom.restore(p.target, p.snapshot);
      return true;
    }
    return false;
  }
}
