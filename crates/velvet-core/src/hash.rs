//! Stable hashing helpers for assets, saves, and manifests.

use sha2::{Digest, Sha256};

/// Compute lowercase hex SHA-256 of bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    hex_encode(&out)
}

/// Compute SHA-256 of a UTF-8 string.
pub fn sha256_str(s: &str) -> String {
    sha256_hex(s.as_bytes())
}

/// FNV-1a 64-bit hash (fast, non-crypto) for cache keys.
pub fn fnv1a64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for b in data {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// FNV-1a of string.
pub fn fnv1a64_str(s: &str) -> u64 {
    fnv1a64(s.as_bytes())
}

/// Combine two u64 hashes (commutative-ish mix).
pub fn mix_u64(a: u64, b: u64) -> u64 {
    let mut x = a ^ b.rotate_left(17);
    x = x.wrapping_mul(0x9e3779b97f4a7c15);
    x ^ (x >> 33)
}

/// Hex encode bytes to lowercase string.
pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// Decode lowercase/uppercase hex; returns None on invalid input.
pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
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
    Some(out)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Rolling content hash for streaming writes.
#[derive(Debug, Clone)]
pub struct RollingSha256 {
    hasher: Sha256,
    bytes: u64,
}

impl Default for RollingSha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl RollingSha256 {
    /// Create.
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    /// Feed bytes.
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
        self.bytes = self.bytes.saturating_add(data.len() as u64);
    }

    /// Bytes absorbed so far.
    pub fn len(&self) -> u64 {
        self.bytes
    }

    /// Whether no bytes.
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }

    /// Finalize to hex (consumes hasher state by cloning digest).
    pub fn finalize_hex(&self) -> String {
        let h = self.hasher.clone().finalize();
        hex_encode(&h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known() {
        // empty string SHA-256
        let h = sha256_str("");
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hex_roundtrip() {
        let data = b"Velvet";
        let h = hex_encode(data);
        assert_eq!(hex_decode(&h).unwrap(), data);
    }

    #[test]
    fn fnv_stable() {
        assert_eq!(fnv1a64_str("a"), fnv1a64(b"a"));
        assert_ne!(fnv1a64_str("a"), fnv1a64_str("b"));
    }

    #[test]
    fn rolling_matches_oneshot() {
        let mut r = RollingSha256::new();
        r.update(b"hel");
        r.update(b"lo");
        assert_eq!(r.finalize_hex(), sha256_str("hello"));
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn mix_changes() {
        assert_ne!(mix_u64(1, 2), mix_u64(1, 3));
    }
}
