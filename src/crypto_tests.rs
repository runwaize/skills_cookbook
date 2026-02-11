#[cfg(test)]
mod tests {
    use crate::crypto::*;
    use crate::types::*;

    #[test]
    fn test_content_hash_computation() {
        let payload = b"test payload data";
        let hash = compute_content_hash(payload);

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Same input should produce same hash
        let hash2 = compute_content_hash(payload);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_content_hash_verification_success() {
        let payload = b"test payload data";
        let expected_hash = compute_content_hash(payload);

        let result = verify_content_hash(payload, &expected_hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_hash_verification_failure() {
        let payload = b"test payload data";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = verify_content_hash(payload, wrong_hash);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("Content hash mismatch"));
        }
    }

    #[test]
    fn test_different_payloads_produce_different_hashes() {
        let payload1 = b"first payload";
        let payload2 = b"second payload";

        let hash1 = compute_content_hash(payload1);
        let hash2 = compute_content_hash(payload2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_empty_payload_hash() {
        let empty_payload = b"";
        let hash = compute_content_hash(empty_payload);

        // SHA-256 of empty string is a known value
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
