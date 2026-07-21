//! Hybrid classical + post-quantum encryption for game tooling.
//!
//! - **Classical:** X25519 ECDH  
//! - **Post-quantum:** ML-KEM-768 (FIPS 203)  
//! - **Combiner:** HKDF-SHA256 over both shared secrets  
//! - **Payload:** ChaCha20-Poly1305 AEAD  
//!
//! Both schemes run **at the same time** (hybrid KEM). Security does not
//! depend on only one remaining unbroken. Not a FIPS product.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use ml_kem::kem::{Decapsulate, Encapsulate};
use ml_kem::{Ciphertext, EncodedSizeUser, KemCore, MlKem768};
use rand_core::OsRng;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};

use crate::{check_len, CryptoError, MAX_CRYPTO_INPUT};

/// Magic for serialized hybrid blobs.
pub const HYBRID_MAGIC: &[u8; 4] = b"VHYB";
/// Format version.
pub const HYBRID_VERSION: u8 = 1;

/// Domain separation for hybrid KDF.
const KDF_SALT: &[u8] = b"VELVET-HYBRID-V1-SALT";
const KDF_INFO: &[u8] = b"VELVET-HYBRID-V1-AEAD-KEY";

type Ek = <MlKem768 as KemCore>::EncapsulationKey;
type Dk = <MlKem768 as KemCore>::DecapsulationKey;

/// Hybrid public key: X25519 + ML-KEM encapsulation key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridPublicKey {
    /// Classical X25519 public key (32 bytes).
    pub classical: [u8; 32],
    /// ML-KEM-768 encapsulation key bytes.
    pub pq: Vec<u8>,
}

/// Hybrid secret key material (keep private).
#[derive(Clone)]
pub struct HybridSecretKey {
    /// Classical static secret.
    pub(crate) classical: StaticSecret,
    /// ML-KEM decapsulation key encoding.
    pub(crate) pq_dk: Vec<u8>,
}

/// Magic for serialized hybrid **secret** keys (distinct from public / sealed).
pub const HYBRID_SECRET_MAGIC: &[u8; 4] = b"VHYS";

impl HybridSecretKey {
    /// Serialize secret key (classical seed + PQ decapsulation key). Treat as confidential.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + 32 + 2 + self.pq_dk.len());
        out.extend_from_slice(HYBRID_SECRET_MAGIC);
        out.push(HYBRID_VERSION);
        out.extend_from_slice(&self.classical.to_bytes());
        let n = self.pq_dk.len() as u16;
        out.extend_from_slice(&n.to_be_bytes());
        out.extend_from_slice(&self.pq_dk);
        out
    }

    /// Parse secret key encoding.
    pub fn from_bytes(b: &[u8]) -> Result<Self, CryptoError> {
        if b.len() < 4 + 1 + 32 + 2 {
            return Err(CryptoError::Msg("hybrid secret key too short".into()));
        }
        if &b[0..4] != HYBRID_SECRET_MAGIC {
            return Err(CryptoError::Msg("bad hybrid secret magic".into()));
        }
        if b[4] != HYBRID_VERSION {
            return Err(CryptoError::Msg("unsupported hybrid secret version".into()));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&b[5..37]);
        let n = u16::from_be_bytes([b[37], b[38]]) as usize;
        if b.len() < 39 + n {
            return Err(CryptoError::Msg("truncated hybrid secret pq".into()));
        }
        Ok(Self {
            classical: StaticSecret::from(seed),
            pq_dk: b[39..39 + n].to_vec(),
        })
    }
}

impl std::fmt::Debug for HybridSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HybridSecretKey { .. }")
    }
}

/// Generated hybrid keypair.
#[derive(Clone)]
pub struct HybridKeyPair {
    /// Public half.
    pub public: HybridPublicKey,
    /// Secret half.
    pub secret: HybridSecretKey,
}

impl std::fmt::Debug for HybridKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridKeyPair")
            .field("public", &self.public)
            .field("secret", &"…")
            .finish()
    }
}

/// Result of hybrid encapsulation (both classical ephemeral + PQ ciphertext).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridKemCiphertext {
    /// Ephemeral X25519 public key.
    pub classical_ephemeral: [u8; 32],
    /// ML-KEM ciphertext.
    pub pq_ciphertext: Vec<u8>,
}

/// Sealed message: hybrid KEM material + AEAD ciphertext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSealed {
    /// KEM components (classical + PQ).
    pub kem: HybridKemCiphertext,
    /// 12-byte nonce.
    pub nonce: [u8; 12],
    /// ChaCha20-Poly1305 ciphertext + tag.
    pub ciphertext: Vec<u8>,
}

impl HybridPublicKey {
    /// Serialize public key (includes both classical and PQ parts).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + 32 + 2 + self.pq.len());
        out.extend_from_slice(HYBRID_MAGIC);
        out.push(HYBRID_VERSION);
        out.extend_from_slice(&self.classical);
        let n = self.pq.len() as u16;
        out.extend_from_slice(&n.to_be_bytes());
        out.extend_from_slice(&self.pq);
        out
    }

    /// Parse public key.
    pub fn from_bytes(b: &[u8]) -> Result<Self, CryptoError> {
        if b.len() < 4 + 1 + 32 + 2 {
            return Err(CryptoError::Msg("hybrid public key too short".into()));
        }
        if &b[0..4] != HYBRID_MAGIC {
            return Err(CryptoError::Msg("bad hybrid public magic".into()));
        }
        if b[4] != HYBRID_VERSION {
            return Err(CryptoError::Msg("unsupported hybrid public version".into()));
        }
        let mut classical = [0u8; 32];
        classical.copy_from_slice(&b[5..37]);
        let n = u16::from_be_bytes([b[37], b[38]]) as usize;
        if b.len() < 39 + n {
            return Err(CryptoError::Msg("truncated hybrid public pq".into()));
        }
        Ok(Self {
            classical,
            pq: b[39..39 + n].to_vec(),
        })
    }

    /// Byte length of public encoding (for hybrid > single-scheme checks).
    pub fn encoded_len(&self) -> usize {
        self.to_bytes().len()
    }
}

impl HybridKemCiphertext {
    /// Serialize KEM ciphertext (both parts).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 2 + self.pq_ciphertext.len());
        out.extend_from_slice(&self.classical_ephemeral);
        let n = self.pq_ciphertext.len() as u16;
        out.extend_from_slice(&n.to_be_bytes());
        out.extend_from_slice(&self.pq_ciphertext);
        out
    }

    /// Parse hybrid KEM ciphertext (classical ephemeral + PQ ciphertext).
    pub fn from_bytes(b: &[u8]) -> Result<Self, CryptoError> {
        if b.len() < 34 {
            return Err(CryptoError::Msg("hybrid kem ct too short".into()));
        }
        let mut classical_ephemeral = [0u8; 32];
        classical_ephemeral.copy_from_slice(&b[0..32]);
        let n = u16::from_be_bytes([b[32], b[33]]) as usize;
        if b.len() < 34 + n {
            return Err(CryptoError::Msg("truncated hybrid kem pq ct".into()));
        }
        Ok(Self {
            classical_ephemeral,
            pq_ciphertext: b[34..34 + n].to_vec(),
        })
    }

    /// Total component size (classical 32 + pq).
    pub fn total_len(&self) -> usize {
        32 + self.pq_ciphertext.len()
    }
}

impl HybridSealed {
    /// Serialize sealed blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let kem = self.kem.to_bytes();
        let mut out = Vec::with_capacity(4 + 1 + 2 + kem.len() + 12 + self.ciphertext.len());
        out.extend_from_slice(HYBRID_MAGIC);
        out.push(HYBRID_VERSION);
        let kn = kem.len() as u16;
        out.extend_from_slice(&kn.to_be_bytes());
        out.extend_from_slice(&kem);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    /// Parse sealed blob.
    pub fn from_bytes(b: &[u8]) -> Result<Self, CryptoError> {
        if b.len() < 4 + 1 + 2 + 34 + 12 {
            return Err(CryptoError::Msg("hybrid sealed too short".into()));
        }
        if &b[0..4] != HYBRID_MAGIC {
            return Err(CryptoError::Msg("bad hybrid sealed magic".into()));
        }
        if b[4] != HYBRID_VERSION {
            return Err(CryptoError::Msg("unsupported hybrid sealed version".into()));
        }
        let kn = u16::from_be_bytes([b[5], b[6]]) as usize;
        if b.len() < 7 + kn + 12 {
            return Err(CryptoError::Msg("truncated hybrid sealed kem".into()));
        }
        let kem = HybridKemCiphertext::from_bytes(&b[7..7 + kn])?;
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&b[7 + kn..7 + kn + 12]);
        let ciphertext = b[7 + kn + 12..].to_vec();
        Ok(Self {
            kem,
            nonce,
            ciphertext,
        })
    }
}

/// Generate hybrid keypair (X25519 + ML-KEM-768).
pub fn hybrid_generate() -> Result<HybridKeyPair, CryptoError> {
    let classical = StaticSecret::random_from_rng(OsRng);
    let classical_pub = X25519Public::from(&classical);

    let (dk, ek) = MlKem768::generate(&mut OsRng);
    let pq = ek.as_bytes().to_vec();
    let pq_dk = dk.as_bytes().to_vec();

    Ok(HybridKeyPair {
        public: HybridPublicKey {
            classical: classical_pub.to_bytes(),
            pq,
        },
        secret: HybridSecretKey { classical, pq_dk },
    })
}

/// Encapsulate to hybrid public key → shared secret (32 bytes after KDF) + KEM ciphertext.
pub fn hybrid_encapsulate(
    pk: &HybridPublicKey,
) -> Result<([u8; 32], HybridKemCiphertext), CryptoError> {
    // Classical: ephemeral X25519
    let eph = StaticSecret::random_from_rng(OsRng);
    let eph_pub = X25519Public::from(&eph);
    let their_pk = X25519Public::from(pk.classical);
    let classical_ss = eph.diffie_hellman(&their_pk);

    // PQ: ML-KEM encapsulate — API returns (ciphertext, shared_secret)
    let ek_enc = ml_kem::Encoded::<Ek>::try_from(pk.pq.as_slice())
        .map_err(|_| CryptoError::Msg("bad ml-kem encapsulation key".into()))?;
    let ek = Ek::from_bytes(&ek_enc);
    let (pq_ct, pq_ss) = ek
        .encapsulate(&mut OsRng)
        .map_err(|_| CryptoError::Msg("ml-kem encapsulate failed".into()))?;

    let kem = HybridKemCiphertext {
        classical_ephemeral: eph_pub.to_bytes(),
        pq_ciphertext: pq_ct.as_slice().to_vec(),
    };

    let key = hybrid_kdf(classical_ss.as_bytes(), pq_ss.as_slice())?;
    Ok((key, kem))
}

/// Decapsulate hybrid KEM ciphertext with secret key → shared secret (32 bytes).
pub fn hybrid_decapsulate(
    sk: &HybridSecretKey,
    ct: &HybridKemCiphertext,
) -> Result<[u8; 32], CryptoError> {
    let their_eph = X25519Public::from(ct.classical_ephemeral);
    let classical_ss = sk.classical.diffie_hellman(&their_eph);

    let dk_enc = ml_kem::Encoded::<Dk>::try_from(sk.pq_dk.as_slice())
        .map_err(|_| CryptoError::Msg("bad ml-kem decapsulation key".into()))?;
    let dk = Dk::from_bytes(&dk_enc);
    // Ciphertext is a fixed-size Array, not EncodedSizeUser
    let pq_ct = Ciphertext::<MlKem768>::try_from(ct.pq_ciphertext.as_slice())
        .map_err(|_| CryptoError::Msg("bad ml-kem ciphertext".into()))?;
    let pq_ss = dk
        .decapsulate(&pq_ct)
        .map_err(|_| CryptoError::Msg("ml-kem decapsulate failed".into()))?;

    hybrid_kdf(classical_ss.as_bytes(), pq_ss.as_slice())
}

fn hybrid_kdf(classical_ss: &[u8], pq_ss: &[u8]) -> Result<[u8; 32], CryptoError> {
    let mut ikm = Vec::with_capacity(classical_ss.len() + pq_ss.len());
    ikm.extend_from_slice(classical_ss);
    ikm.extend_from_slice(pq_ss);
    let hk = Hkdf::<Sha256>::new(Some(KDF_SALT), &ikm);
    let mut okm = [0u8; 32];
    hk.expand(KDF_INFO, &mut okm)
        .map_err(|_| CryptoError::Msg("hkdf expand failed".into()))?;
    Ok(okm)
}

/// Hybrid seal: encapsulate + AEAD encrypt (classical and PQ used together).
pub fn hybrid_seal(pk: &HybridPublicKey, plaintext: &[u8]) -> Result<HybridSealed, CryptoError> {
    check_len(plaintext.len())?;
    let (key, kem) = hybrid_encapsulate(pk)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| CryptoError::Msg(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: HYBRID_MAGIC,
            },
        )
        .map_err(|_| CryptoError::Msg("aead encrypt failed".into()))?;
    Ok(HybridSealed {
        kem,
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Hybrid open: decapsulate + AEAD decrypt.
pub fn hybrid_open(sk: &HybridSecretKey, sealed: &HybridSealed) -> Result<Vec<u8>, CryptoError> {
    if sealed.ciphertext.len() > MAX_CRYPTO_INPUT + 16 {
        return Err(CryptoError::TooLarge {
            max: MAX_CRYPTO_INPUT,
        });
    }
    let key = hybrid_decapsulate(sk, &sealed.kem)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let nonce = Nonce::from_slice(&sealed.nonce);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: &sealed.ciphertext,
                aad: HYBRID_MAGIC,
            },
        )
        .map_err(|_| CryptoError::Msg("aead decrypt failed".into()))
}

/// Fingerprint public key for logging (hex of sha256 of encoding).
pub fn hybrid_public_fingerprint(pk: &HybridPublicKey) -> Result<String, CryptoError> {
    let b = pk.to_bytes();
    crate::hash_sha256_hex(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_kem_shared_secret_matches() {
        let kp = hybrid_generate().unwrap();
        let (ss1, ct) = hybrid_encapsulate(&kp.public).unwrap();
        let ss2 = hybrid_decapsulate(&kp.secret, &ct).unwrap();
        assert_eq!(ss1, ss2);
        // Hybrid carries both classical (32) and PQ ciphertext (>> 32)
        assert!(ct.total_len() > 32 + 100, "pq ciphertext must be present");
        assert_eq!(ct.classical_ephemeral.len(), 32);
        assert!(!ct.pq_ciphertext.is_empty());
    }

    #[test]
    fn hybrid_public_larger_than_classical_alone() {
        let kp = hybrid_generate().unwrap();
        let encoded = kp.public.to_bytes();
        // classical alone is 32 bytes; hybrid encoding >> 32
        assert!(
            encoded.len() > 32 + 64,
            "hybrid public must include PQ material, len={}",
            encoded.len()
        );
        assert!(kp.public.pq.len() > 64);
        let round = HybridPublicKey::from_bytes(&encoded).unwrap();
        assert_eq!(round.classical, kp.public.classical);
        assert_eq!(round.pq, kp.public.pq);
    }

    #[test]
    fn hybrid_seal_open_roundtrip() {
        let kp = hybrid_generate().unwrap();
        let msg = b"Velvet hybrid PQ+classical seal";
        let sealed = hybrid_seal(&kp.public, msg).unwrap();
        // sealed blob must exceed pure classical seal size heuristics
        let blob = sealed.to_bytes();
        assert!(blob.len() > 32 + 12 + msg.len() + 16);
        let open = hybrid_open(&kp.secret, &sealed).unwrap();
        assert_eq!(open, msg);

        let parsed = HybridSealed::from_bytes(&blob).unwrap();
        let open2 = hybrid_open(&kp.secret, &parsed).unwrap();
        assert_eq!(open2, msg);
    }

    #[test]
    fn hybrid_wrong_key_fails() {
        let a = hybrid_generate().unwrap();
        let b = hybrid_generate().unwrap();
        let sealed = hybrid_seal(&a.public, b"secret").unwrap();
        assert!(hybrid_open(&b.secret, &sealed).is_err());
    }

    #[test]
    fn fingerprint_hex() {
        let kp = hybrid_generate().unwrap();
        let fp = hybrid_public_fingerprint(&kp.public).unwrap();
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn secret_key_roundtrip() {
        let kp = hybrid_generate().unwrap();
        let raw = kp.secret.to_bytes();
        assert!(raw.starts_with(HYBRID_SECRET_MAGIC));
        let sk = HybridSecretKey::from_bytes(&raw).unwrap();
        let msg = b"secret-roundtrip";
        let sealed = hybrid_seal(&kp.public, msg).unwrap();
        assert_eq!(hybrid_open(&sk, &sealed).unwrap(), msg);
    }
}
