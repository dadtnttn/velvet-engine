//! # velvet-crypto
//!
//! Game-oriented crypto/codec **tools** for VS2:
//! - Hash / HMAC / RNG / base64 / hex
//! - **Hybrid classical + post-quantum** seal (X25519 + ML-KEM-768 + ChaCha20-Poly1305)
//!
//! Not a FIPS product. Hard limits apply to reduce DoS from scripts.

#![deny(missing_docs)]

mod hybrid;

pub use hybrid::{
    hybrid_decapsulate, hybrid_encapsulate, hybrid_generate, hybrid_open,
    hybrid_public_fingerprint, hybrid_seal, HybridKemCiphertext, HybridKeyPair, HybridPublicKey,
    HybridSealed, HybridSecretKey, HYBRID_MAGIC, HYBRID_SECRET_MAGIC, HYBRID_VERSION,
};

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

/// Max input bytes for hash/hmac/codec/seal plaintext.
pub const MAX_CRYPTO_INPUT: usize = 1 * 1024 * 1024;
/// Max random_bytes length.
pub const MAX_RANDOM_BYTES: usize = 64 * 1024;

/// Crypto tool error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Input too large
    #[error("crypto input too large (max {max})")]
    TooLarge {
        /// Cap
        max: usize,
    },
    /// Other
    #[error("crypto: {0}")]
    Msg(String),
}

pub(crate) fn check_len(n: usize) -> Result<(), CryptoError> {
    if n > MAX_CRYPTO_INPUT {
        return Err(CryptoError::TooLarge {
            max: MAX_CRYPTO_INPUT,
        });
    }
    Ok(())
}

/// SHA-256 digest (32 bytes).
pub fn hash_sha256(data: &[u8]) -> Result<[u8; 32], CryptoError> {
    check_len(data.len())?;
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    Ok(arr)
}

/// SHA-256 as lowercase hex.
pub fn hash_sha256_hex(data: &[u8]) -> Result<String, CryptoError> {
    Ok(hex_encode(&hash_sha256(data)?))
}

/// HMAC-SHA256 (32 bytes).
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> Result<[u8; 32], CryptoError> {
    check_len(key.len())?;
    check_len(msg.len())?;
    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|e| CryptoError::Msg(e.to_string()))?;
    mac.update(msg);
    let out = mac.finalize().into_bytes();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    Ok(arr)
}

/// HMAC-SHA256 hex.
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> Result<String, CryptoError> {
    Ok(hex_encode(&hmac_sha256(key, msg)?))
}

/// Cryptographic random bytes (length capped).
pub fn random_bytes(n: usize) -> Result<Vec<u8>, CryptoError> {
    if n > MAX_RANDOM_BYTES {
        return Err(CryptoError::TooLarge {
            max: MAX_RANDOM_BYTES,
        });
    }
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| CryptoError::Msg(e.to_string()))?;
    Ok(buf)
}

/// Constant-time equality for equal-length slices.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut v = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        v |= x ^ y;
    }
    v == 0
}

/// Hex encode (lowercase).
pub fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// Hex decode.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    check_len(s.len())?;
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(CryptoError::Msg("hex length must be even".into()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_val(c: u8) -> Result<u8, CryptoError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(CryptoError::Msg("invalid hex".into())),
    }
}

/// Base64 encode (standard alphabet, no pad option — with padding).
pub fn base64_encode(data: &[u8]) -> Result<String, CryptoError> {
    check_len(data.len())?;
    const T: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < data.len() {
            out.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(T[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    Ok(out)
}

/// Base64 decode.
pub fn base64_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    check_len(s.len())?;
    let s = s.trim().as_bytes();
    let mut vals = Vec::with_capacity(s.len());
    for &c in s {
        if c == b'=' {
            break;
        }
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'\n' | b'\r' | b' ' => continue,
            _ => return Err(CryptoError::Msg("invalid base64".into())),
        };
        vals.push(v);
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < vals.len() {
        let a = vals[i];
        let b = vals[i + 1];
        out.push((a << 2) | (b >> 4));
        if i + 2 < vals.len() {
            let c = vals[i + 2];
            out.push(((b & 15) << 4) | (c >> 2));
            if i + 3 < vals.len() {
                let d = vals[i + 3];
                out.push(((c & 3) << 6) | d);
            }
        }
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_known() {
        // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let h = hash_sha256_hex(b"").unwrap();
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hmac_and_eq() {
        let a = hmac_sha256(b"key", b"msg").unwrap();
        let b = hmac_sha256(b"key", b"msg").unwrap();
        assert!(constant_time_eq(&a, &b));
        let c = hmac_sha256(b"key", b"msg2").unwrap();
        assert!(!constant_time_eq(&a, &c));
    }

    #[test]
    fn base64_roundtrip() {
        let s = base64_encode(b"Velvet").unwrap();
        let back = base64_decode(&s).unwrap();
        assert_eq!(back, b"Velvet");
    }

    #[test]
    fn hex_roundtrip() {
        let h = hex_encode(b"\x00\xff");
        assert_eq!(h, "00ff");
        assert_eq!(hex_decode(&h).unwrap(), vec![0, 255]);
    }

    #[test]
    fn random_bytes_len() {
        let r = random_bytes(16).unwrap();
        assert_eq!(r.len(), 16);
    }

    #[test]
    fn too_large_rejected() {
        let big = vec![0u8; MAX_CRYPTO_INPUT + 1];
        assert!(hash_sha256(&big).is_err());
    }
}
