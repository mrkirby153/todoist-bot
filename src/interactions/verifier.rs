use anyhow::{Result, anyhow};
use ed25519_dalek::VerifyingKey;
use thiserror::Error;

fn parse_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    if s.len() != N * 2 {
        return None;
    }
    let mut result = [0; N];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        result[i] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(result)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid signature")]
    Signature,
    #[error("Invalid public key")]
    PublicKey(
        #[from]
        #[source]
        ed25519_dalek::SignatureError,
    ),
    #[error("Invalid public key format")]
    PublicKeyFormat,
}

/// Verifies Discord interaction signatures.
pub struct Verifier {
    key: VerifyingKey,
}

impl Verifier {
    /// Creates a new Verifier from a hex-encoded public key.
    pub fn try_new(public_key: &str) -> Result<Self> {
        let public_key = parse_hex::<32>(public_key).ok_or(anyhow!(Error::PublicKeyFormat))?;
        Ok(Self {
            key: VerifyingKey::from_bytes(&public_key).map_err(|e| anyhow!(Error::PublicKey(e)))?,
        })
    }

    /// Verifies a signature given the timestamp and body.
    pub fn verify(&self, signature: &str, timestamp: &str, body: &[u8]) -> Result<()> {
        use ed25519_dalek::Verifier as _;
        let sig_bytes = parse_hex(signature).ok_or(Error::Signature)?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        let to_verify = [timestamp.as_bytes(), body].concat();
        self.key
            .verify(&to_verify, &signature)
            .map_err(|_| anyhow!(Error::Signature))
    }
}
