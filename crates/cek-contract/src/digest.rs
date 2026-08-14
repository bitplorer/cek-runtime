//! Stable digests for Results and sealed-args binds.
//!
//! Digests are **deterministic** over a canonical JSON encoding of the
//! relevant fields. Algorithm id is embedded so future algorithms can
//! coexist without silent reinterpretation.
//!
//! Format: `cek1:<hex>` where the hex is SHA-256 over canonical bytes.

use crate::Op;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Algorithm prefix for digests produced by this contract generation.
pub const DIGEST_ALG: &str = "cek1";

/// Compute SHA-256 hex of `bytes` without external deps (pure Rust FIPS-180-ish minimal).
///
/// We use a compact standalone SHA-256 so the contract crate stays dependency-light
/// and digests stay stable across platforms.
fn sha256_hex(bytes: &[u8]) -> String {
    let hash = sha256(bytes);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256(message: &[u8]) -> [u8; 32] {
    // Minimal SHA-256 implementation for stable digests.
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (message.len() as u64) * 8;
    let mut msg = message.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
    }
    out
}

/// Canonical JSON bytes for digest input (BTreeMap order is stable).
fn canonical_bytes(v: &Value) -> Vec<u8> {
    // serde_json preserves BTreeMap key order; we build only ordered maps.
    serde_json::to_vec(v).unwrap_or_default()
}

/// Digest of sealed-args bind material.
///
/// Callers pass the **sealed subset** of Intent.args (or a Host-defined
/// sealed projection). Host compares this to `Cap.sealed_args_bind`.
pub fn sealed_args_digest(sealed: &BTreeMap<String, Value>) -> String {
    let v = Value::Object(sealed.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    format!("{DIGEST_ALG}:{}", sha256_hex(&canonical_bytes(&v)))
}

/// Digest of authorized Ops list (projection stability).
pub fn ops_digest(ops: &[Op]) -> String {
    let arr: Vec<Value> = ops
        .iter()
        .map(|op| {
            json!({
                "ns": op.ns,
                "name": op.name,
                "payload": op.payload,
            })
        })
        .collect();
    format!(
        "{DIGEST_ALG}:{}",
        sha256_hex(&canonical_bytes(&Value::Array(arr)))
    )
}

/// Full Result digest over kind + ops + error (stable idempotent replay token).
pub fn result_digest(kind: &str, ops: &[Op], error: Option<&str>) -> String {
    let v = json!({
        "kind": kind,
        "ops": ops.iter().map(|op| json!({
            "ns": op.ns,
            "name": op.name,
            "payload": op.payload,
        })).collect::<Vec<_>>(),
        "error": error,
    });
    format!("{DIGEST_ALG}:{}", sha256_hex(&canonical_bytes(&v)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline;

    #[test]
    fn digests_stable() {
        let ops = vec![baseline::kv_set("a", json!(1))];
        let d1 = ops_digest(&ops);
        let d2 = ops_digest(&ops);
        assert_eq!(d1, d2);
        assert!(d1.starts_with("cek1:"));
    }

    #[test]
    fn sealed_order_independent_via_btreemap() {
        let mut a = BTreeMap::new();
        a.insert("k".into(), json!(1));
        a.insert("m".into(), json!(2));
        let mut b = BTreeMap::new();
        b.insert("m".into(), json!(2));
        b.insert("k".into(), json!(1));
        assert_eq!(sealed_args_digest(&a), sealed_args_digest(&b));
    }

    #[test]
    fn sha256_known_answers() {
        // FIPS 180-2 / RFC 6234 fixtures — digest stability depends on this.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn empty_ops_digest_is_cek1() {
        let d = ops_digest(&[]);
        assert!(d.starts_with("cek1:"));
        assert_eq!(d, ops_digest(&[]));
    }

    #[test]
    fn result_digest_distinguishes_kind() {
        let ops = vec![baseline::kv_set("a", json!(1))];
        let ok = result_digest("ok", &ops, None);
        let refuse = result_digest("authority_refusal", &[], Some("no"));
        let disp = result_digest("dispatch_error", &[], Some("miss"));
        assert_ne!(ok, refuse);
        assert_ne!(ok, disp);
        assert_ne!(refuse, disp);
        assert!(ok.starts_with("cek1:"));
    }

    #[test]
    fn multi_op_digest_order_sensitive() {
        let a = baseline::kv_set("a", json!(1));
        let b = baseline::kv_delete("a");
        assert_ne!(ops_digest(&[a.clone(), b.clone()]), ops_digest(&[b, a]));
    }
}
