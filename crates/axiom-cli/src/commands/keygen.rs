use rand::RngCore;
use sha2::{Digest, Sha256};

/// Generate a cryptographically random API key and print:
///   - the plaintext key (for use as `X-Axiom-Key` header)
///   - the `sha256:...` hash (for use in `axiom.yaml` or `POST /v1/keys`)
///
/// §6.2.1 [R3-3]
pub fn run(role: &str, description: Option<&str>) {
    // 32 bytes = 64 hex chars — sufficient entropy
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let key = hex::encode(raw);

    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let hash = hex::encode(hasher.finalize());

    println!("# Axiom API key — {}", description.unwrap_or(role));
    println!("#");
    println!("# Add to axiom.yaml:");
    println!("# keys:");
    println!("#   - id: my-key");
    println!("#     role: {role}");
    println!("#     hash: \"sha256:{hash}\"");
    println!();
    println!("Key (X-Axiom-Key header value — shown once, not stored):");
    println!("{key}");
    println!();
    println!("Hash (axiom.yaml / POST /v1/keys):");
    println!("sha256:{hash}");
}
