//! Host-policy Cap signatures (HMAC-SHA256 and Ed25519).
//!
//! Not law. Peer never signs. Prefixes:
//! - `cek1:` HMAC-SHA256 (shared secret)
//! - `ed25519:` Ed25519 over the same `cap_sign_bytes`

use crate::{HostError, HostResult};
use cek_contract::{cap_sign_bytes, cap_signature, Cap, SIG_ED25519};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Parse 64 hex chars into 32 bytes.
pub fn parse_hex32(hex: &str) -> HostResult<[u8; 32]> {
    parse_hex_n::<32>(hex)
}

fn parse_hex_n<const N: usize>(hex: &str) -> HostResult<[u8; N]> {
    let hex = hex.trim();
    if hex.len() != N * 2 {
        return Err(HostError::Authority(format!(
            "expected {} hex chars, got {}",
            N * 2,
            hex.len()
        )));
    }
    let mut out = [0u8; N];
    for i in 0..N {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| HostError::Authority(format!("hex: {e}")))?;
    }
    Ok(out)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Ed25519 signing key from a 32-byte seed (RFC 8032 secret).
pub fn signing_key(seed: &[u8; 32]) -> SigningKey {
    SigningKey::from_bytes(seed)
}

/// Public key bytes for `seed`.
pub fn public_key(seed: &[u8; 32]) -> [u8; 32] {
    signing_key(seed).verifying_key().to_bytes()
}

/// Attach `ed25519:<hex>` over authority fields.
pub fn attach_ed25519(sk: &SigningKey, mut cap: Cap) -> Cap {
    let sig = sk.sign(&cap_sign_bytes(&cap));
    cap.sig = Some(format!("{SIG_ED25519}:{}", to_hex(&sig.to_bytes())));
    cap
}

/// True if any trusted public key verifies `cap.sig`.
pub fn ed25519_valid(trusted: &[VerifyingKey], cap: &Cap) -> bool {
    let Some(raw) = cap.sig.as_deref() else {
        return false;
    };
    let Some(hex) = raw.strip_prefix("ed25519:") else {
        return false;
    };
    let Ok(bytes) = parse_hex_n::<64>(hex) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&bytes) else {
        return false;
    };
    let msg = cap_sign_bytes(cap);
    trusted.iter().any(|vk| vk.verify(&msg, &sig).is_ok())
}

/// HMAC attach (existing `cek1:` MAC).
pub fn attach_hmac(key: &[u8; 32], mut cap: Cap) -> Cap {
    cap.sig = Some(cap_signature(key, &cap));
    cap
}

/// Decode a verifying key.
pub fn verifying_key(bytes: &[u8; 32]) -> HostResult<VerifyingKey> {
    VerifyingKey::from_bytes(bytes)
        .map_err(|_| HostError::Authority("invalid Ed25519 public key".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc8032_case1_empty_message() {
        // RFC 8032 Test 1 secret / public / signature over empty msg.
        let seed = parse_hex32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
            .unwrap();
        let sk = signing_key(&seed);
        assert_eq!(
            to_hex(&sk.verifying_key().to_bytes()),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
        let sig = sk.sign(b"");
        assert_eq!(
            to_hex(&sig.to_bytes()),
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );
    }
}
