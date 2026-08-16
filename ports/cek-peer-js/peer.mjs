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
    this.roots = [{ tag: "div", attrs: { id: "root" }, children: [] }];
  }

  morph(target, patch) {
    return replaceAt(this.roots, target, patch);
  }

  restore(target, snapshot) {
    return this.morph(target, snapshot);
  }

  get(target) {
    return findAt(this.roots, target);
  }

  insertChild(parent, child) {
    const n = findAt(this.roots, parent);
    if (!n) return false;
    if (!Array.isArray(n.children)) n.children = [];
    n.children.push(child);
    return true;
  }

  remove(target) {
    return takeAt(this.roots, target);
  }

  setText(target, text) {
    const n = findAt(this.roots, target);
    if (!n) return false;
    n.text = text;
    return true;
  }

  setAttr(target, key, value) {
    const n = findAt(this.roots, target);
    if (!n) return false;
    n.attrs = n.attrs || {};
    n.attrs[key] = value;
    return true;
  }

  html() {
    return this.roots.map(render).join("");
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

function norm(t) {
  return t.startsWith("#") ? t.slice(1) : t;
}

function parsePath(t) {
  if (!t.startsWith("/")) return null;
  const rest = t.slice(1);
  if (!rest) return [];
  const parts = rest.split("/");
  const out = [];
  for (const p of parts) {
    if (!/^\d+$/.test(p)) return null;
    out.push(Number(p));
  }
  return out;
}

function getPath(nodes, path) {
  if (!path.length) return undefined;
  if (path.length === 1) return nodes[path[0]];
  const n = nodes[path[0]];
  return n && Array.isArray(n.children) ? getPath(n.children, path.slice(1)) : undefined;
}

function setPath(nodes, path, patch) {
  if (!path.length) return false;
  if (path.length === 1) {
    if (path[0] >= nodes.length) return false;
    nodes[path[0]] = patch;
    return true;
  }
  const n = nodes[path[0]];
  return n && Array.isArray(n.children) ? setPath(n.children, path.slice(1), patch) : false;
}

function replaceAt(nodes, target, patch) {
  const path = parsePath(target);
  if (path) return setPath(nodes, path, patch);
  const id = norm(target);
  for (let i = 0; i < nodes.length; i++) {
    if (idOf(nodes[i]) === id) {
      nodes[i] = patch;
      return true;
    }
    if (Array.isArray(nodes[i].children) && replaceAt(nodes[i].children, target, patch)) {
      return true;
    }
  }
  return false;
}

function findAt(nodes, target) {
  const path = parsePath(target);
  if (path) return getPath(nodes, path);
  const id = norm(target);
  for (const n of nodes) {
    if (idOf(n) === id) return n;
    if (Array.isArray(n.children)) {
      const hit = findAt(n.children, target);
      if (hit) return hit;
    }
  }
  return undefined;
}

function takeAt(nodes, target) {
  const path = parsePath(target);
  if (path) {
    if (path.length === 1) return nodes.splice(path[0], 1)[0];
    const n = nodes[path[0]];
    return n && Array.isArray(n.children) ? takeAt(n.children, "/" + path.slice(1).join("/")) : undefined;
  }
  const id = norm(target);
  const i = nodes.findIndex((n) => idOf(n) === id);
  if (i >= 0) return nodes.splice(i, 1)[0];
  for (const n of nodes) {
    if (Array.isArray(n.children)) {
      const hit = takeAt(n.children, target);
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

function render(n) {
  const tag = n.tag || "div";
  const attrs = n.attrs
    ? Object.keys(n.attrs)
        .sort()
        .map((k) => ` ${k}="${n.attrs[k]}"`)
        .join("")
    : "";
  const text = n.text || "";
  const kids = Array.isArray(n.children) ? n.children.map(render).join("") : "";
  return `<${tag}${attrs}>${text}${kids}</${tag}>`;
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
      if (typeof op.name === "string" && op.name.includes(".")) {
        failed.push(op);
        if (this.unknownOpPolicy === "fail_batch") abort = true;
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
