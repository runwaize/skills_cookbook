use crate::error::{RelayError, Result};
use crate::types::Artifact;
use ring::signature::{self, UnparsedPublicKey};
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose};

/// Verify artifact signature and content hash
pub fn verify_artifact(artifact: &Artifact, public_key_pem: &str) -> Result<()> {
    // Step 1: Verify content hash
    verify_content_hash(&artifact.payload, &artifact.content_hash)?;

    // Step 2: Verify signature
    verify_signature(artifact, public_key_pem)?;

    Ok(())
}

/// Compute and verify content hash
pub fn verify_content_hash(payload: &[u8], expected_hash: &str) -> Result<()> {
    let computed_hash = compute_content_hash(payload);

    if computed_hash != expected_hash {
        return Err(RelayError::Verification(format!(
            "Content hash mismatch: expected {}, got {}",
            expected_hash, computed_hash
        )));
    }

    Ok(())
}

/// Compute SHA-256 hash of payload
pub fn compute_content_hash(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}

/// Verify artifact signature using public key
fn verify_signature(artifact: &Artifact, public_key_pem: &str) -> Result<()> {
    // Check expiration if set
    if let Some(expires_at) = artifact.signature.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(RelayError::InvalidSignature(format!(
                "Signature expired at {}",
                expires_at
            )));
        }
    }

    // Construct message to verify (same as signed)
    let message = construct_signature_message(
        &artifact.content_hash,
        &artifact.skill_id,
        &artifact.skill_version_id,
        &artifact.artifact_type,
        &artifact.signature.issued_at,
        &artifact.signature.expires_at,
    );

    // Decode signature from base64
    let signature_bytes = general_purpose::STANDARD.decode(&artifact.signature.signature)
        .map_err(|e| RelayError::InvalidSignature(format!("Invalid signature encoding: {}", e)))?;

    // Parse public key
    let public_key_der = parse_pem_public_key(public_key_pem)?;

    // Verify signature based on algorithm
    match artifact.signature.algorithm.as_str() {
        "ED25519" => {
            let public_key = UnparsedPublicKey::new(&signature::ED25519, &public_key_der);
            public_key
                .verify(message.as_bytes(), &signature_bytes)
                .map_err(|_| RelayError::InvalidSignature("Signature verification failed".to_string()))?;
        }
        "RSA_PSS_SHA256" => {
            let public_key = UnparsedPublicKey::new(&signature::RSA_PSS_2048_8192_SHA256, &public_key_der);
            public_key
                .verify(message.as_bytes(), &signature_bytes)
                .map_err(|_| RelayError::InvalidSignature("Signature verification failed".to_string()))?;
        }
        algo => {
            return Err(RelayError::InvalidSignature(format!(
                "Unsupported signature algorithm: {}",
                algo
            )));
        }
    }

    tracing::debug!("Signature verified successfully for artifact {}", artifact.artifact_id);

    Ok(())
}

/// Construct the message that was signed
fn construct_signature_message(
    content_hash: &str,
    skill_id: &str,
    skill_version: &str,
    artifact_type: &crate::types::ArtifactType,
    issued_at: &chrono::DateTime<chrono::Utc>,
    expires_at: &Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    let artifact_type_str = match artifact_type {
        crate::types::ArtifactType::Prompt => "prompt",
        crate::types::ArtifactType::McpTool => "mcp-tool",
        crate::types::ArtifactType::OpenclawSkill => "openclaw-skill",
        crate::types::ArtifactType::Generic => "generic",
    };

    let expires_str = expires_at
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{}:{}:{}:{}:{}:{}",
        content_hash,
        skill_id,
        skill_version,
        artifact_type_str,
        issued_at.to_rfc3339(),
        expires_str
    )
}

/// Parse PEM-encoded public key to DER format
fn parse_pem_public_key(pem: &str) -> Result<Vec<u8>> {
    // Remove PEM headers and decode base64
    let pem_lines: Vec<&str> = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();

    let pem_data = pem_lines.join("");

    general_purpose::STANDARD.decode(&pem_data)
        .map_err(|e| RelayError::InvalidSignature(format!("Invalid PEM encoding: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash() {
        let payload = b"test payload";
        let hash = compute_content_hash(payload);
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_verify_content_hash_success() {
        let payload = b"test payload";
        let hash = compute_content_hash(payload);
        assert!(verify_content_hash(payload, &hash).is_ok());
    }

    #[test]
    fn test_verify_content_hash_failure() {
        let payload = b"test payload";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_content_hash(payload, wrong_hash).is_err());
    }
}
