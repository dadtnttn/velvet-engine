//! `velvet crypto hybrid …` — hybrid classical + post-quantum seal tooling.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_crypto::{
    hybrid_generate, hybrid_open, hybrid_public_fingerprint, hybrid_seal, HybridPublicKey,
    HybridSealed, HybridSecretKey,
};

/// Generate hybrid keypair (X25519 + ML-KEM-768) to files.
pub fn cmd_hybrid_keygen(public_out: PathBuf, secret_out: PathBuf) -> Result<()> {
    let kp = hybrid_generate().map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(&public_out, kp.public.to_bytes())
        .with_context(|| format!("write {}", public_out.display()))?;
    std::fs::write(&secret_out, kp.secret.to_bytes())
        .with_context(|| format!("write {}", secret_out.display()))?;
    let fp = hybrid_public_fingerprint(&kp.public).map_err(|e| anyhow::anyhow!(e))?;
    println!(
        "hybrid keygen ok fingerprint={fp} public={} ({} B) secret={} ({} B)",
        public_out.display(),
        kp.public.to_bytes().len(),
        secret_out.display(),
        kp.secret.to_bytes().len(),
    );
    Ok(())
}

/// Seal plaintext file to hybrid ciphertext blob.
pub fn cmd_hybrid_seal(public_key: PathBuf, input: PathBuf, output: PathBuf) -> Result<()> {
    let pk_bytes =
        std::fs::read(&public_key).with_context(|| format!("read {}", public_key.display()))?;
    let pk = HybridPublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!(e))?;
    let plain = std::fs::read(&input).with_context(|| format!("read {}", input.display()))?;
    let sealed = hybrid_seal(&pk, &plain).map_err(|e| anyhow::anyhow!(e))?;
    let blob = sealed.to_bytes();
    std::fs::write(&output, &blob).with_context(|| format!("write {}", output.display()))?;
    println!(
        "hybrid seal ok {} → {} ({} B plaintext, {} B sealed)",
        input.display(),
        output.display(),
        plain.len(),
        blob.len()
    );
    Ok(())
}

/// Open hybrid sealed blob with secret key.
pub fn cmd_hybrid_open(secret_key: PathBuf, input: PathBuf, output: PathBuf) -> Result<()> {
    let sk_bytes =
        std::fs::read(&secret_key).with_context(|| format!("read {}", secret_key.display()))?;
    let sk = HybridSecretKey::from_bytes(&sk_bytes).map_err(|e| anyhow::anyhow!(e))?;
    let blob = std::fs::read(&input).with_context(|| format!("read {}", input.display()))?;
    let sealed = HybridSealed::from_bytes(&blob).map_err(|e| anyhow::anyhow!(e))?;
    let plain = hybrid_open(&sk, &sealed).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(&output, &plain).with_context(|| format!("write {}", output.display()))?;
    println!(
        "hybrid open ok {} → {} ({} B)",
        input.display(),
        output.display(),
        plain.len()
    );
    Ok(())
}

/// Print fingerprint of a hybrid public key file.
pub fn cmd_hybrid_fingerprint(public_key: PathBuf) -> Result<()> {
    let pk_bytes =
        std::fs::read(&public_key).with_context(|| format!("read {}", public_key.display()))?;
    let pk = HybridPublicKey::from_bytes(&pk_bytes).map_err(|e| anyhow::anyhow!(e))?;
    if pk.pq.is_empty() {
        bail!("public key has empty PQ material");
    }
    let fp = hybrid_public_fingerprint(&pk).map_err(|e| anyhow::anyhow!(e))?;
    println!(
        "fingerprint={fp} classical=32 pq={} total={}",
        pk.pq.len(),
        pk.encoded_len()
    );
    Ok(())
}
