//! Ephemeral ECDSA key generation and management

use p256::{
    ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey},
    SecretKey,
};
use sha2::{Digest, Sha256};

/// Trait for any identity that can sign documents
pub trait SigningIdentity {
    /// Get the public key as DER-encoded bytes
    fn public_key_der(&self) -> Vec<u8>;

    /// Get the public key as hex string
    fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_der())
    }

    /// Sign raw data and return DER-encoded signature
    fn sign(&self, data: &[u8]) -> Vec<u8>;

    /// Sign data with SHA-256 pre-hashing
    fn sign_prehashed(&self, hash: &[u8; 32]) -> Vec<u8>;

    /// Verify a signature
    fn verify(&self, data: &[u8], signature: &[u8]) -> bool;

    /// Get certificate DER bytes if this is a certificate-based identity
    fn certificate_der(&self) -> Option<&[u8]> {
        None
    }

    /// Get the signer name (from certificate subject or fallback)
    fn signer_name(&self) -> Option<&str> {
        None
    }
}

/// An ephemeral identity for signing documents
pub struct EphemeralIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl EphemeralIdentity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        let secret_key = SecretKey::random(&mut rand_core::OsRng);
        let signing_key = SigningKey::from(&secret_key);
        let verifying_key = VerifyingKey::from(&signing_key);

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Get the public key as DER-encoded bytes (for embedding in signatures)
    pub fn public_key_der(&self) -> Vec<u8> {
        // Return the SEC1-encoded public key (uncompressed)
        self.verifying_key
            .to_encoded_point(false)
            .as_bytes()
            .to_vec()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_der())
    }

    /// Sign raw data and return DER-encoded signature
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(data);
        signature.to_der().as_bytes().to_vec()
    }

    /// Sign data with SHA-256 pre-hashing
    pub fn sign_prehashed(&self, hash: &[u8; 32]) -> Vec<u8> {
        // Create a digest from the pre-computed hash
        let signature: Signature = self.signing_key.sign(hash);
        signature.to_der().as_bytes().to_vec()
    }

    /// Sign and return hex-encoded signature
    pub fn sign_hex(&self, data: &[u8]) -> String {
        hex::encode(self.sign(data))
    }

    /// Verify a signature
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        use p256::ecdsa::signature::Verifier;

        if let Ok(sig) = Signature::from_der(signature) {
            self.verifying_key.verify(data, &sig).is_ok()
        } else {
            false
        }
    }

    /// Export the private key (for temporary storage in IndexedDB)
    /// WARNING: Handle with care - this exposes the private key
    pub fn export_private_key(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    /// Import from previously exported private key
    pub fn from_private_key(bytes: &[u8]) -> Result<Self, String> {
        let secret_key =
            SecretKey::from_slice(bytes).map_err(|e| format!("Invalid private key: {}", e))?;
        let signing_key = SigningKey::from(&secret_key);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }
}

impl SigningIdentity for EphemeralIdentity {
    fn public_key_der(&self) -> Vec<u8> {
        self.verifying_key
            .to_encoded_point(false)
            .as_bytes()
            .to_vec()
    }

    fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(data);
        signature.to_der().as_bytes().to_vec()
    }

    fn sign_prehashed(&self, hash: &[u8; 32]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(hash);
        signature.to_der().as_bytes().to_vec()
    }

    fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        use p256::ecdsa::signature::Verifier;

        if let Ok(sig) = Signature::from_der(signature) {
            self.verifying_key.verify(data, &sig).is_ok()
        } else {
            false
        }
    }
}

/// Hash data using SHA-256
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash data and return as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let identity = EphemeralIdentity::generate();
        let public_key = identity.public_key_der();

        // P-256 uncompressed public key is 65 bytes (0x04 prefix + 32 bytes X + 32 bytes Y)
        assert_eq!(public_key.len(), 65);
        assert_eq!(public_key[0], 0x04);
    }

    #[test]
    fn test_sign_verify() {
        let identity = EphemeralIdentity::generate();
        let message = b"Hello, DocSign!";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));

        // Verify fails with wrong message
        assert!(!identity.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_export_import() {
        let identity = EphemeralIdentity::generate();
        let message = b"Test message";
        let signature = identity.sign(message);

        // Export and re-import
        let exported = identity.export_private_key();
        let restored = EphemeralIdentity::from_private_key(&exported).unwrap();

        // Should verify with restored identity
        assert!(restored.verify(message, &signature));

        // Public keys should match
        assert_eq!(identity.public_key_der(), restored.public_key_der());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Any message can be signed and the signature verifies
        #[test]
        fn sign_verify_roundtrip(message in prop::collection::vec(any::<u8>(), 0..1024)) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&message);
            prop_assert!(identity.verify(&message, &signature));
        }

        /// Property: Signatures don't verify with different messages
        #[test]
        fn signature_message_binding(
            msg1 in prop::collection::vec(any::<u8>(), 1..512),
            msg2 in prop::collection::vec(any::<u8>(), 1..512),
        ) {
            prop_assume!(msg1 != msg2);
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&msg1);
            prop_assert!(!identity.verify(&msg2, &signature));
        }

        /// Property: Export/import preserves signing capability
        #[test]
        fn export_import_roundtrip(message in prop::collection::vec(any::<u8>(), 0..512)) {
            let identity = EphemeralIdentity::generate();
            let exported = identity.export_private_key();
            let restored = EphemeralIdentity::from_private_key(&exported).unwrap();

            // Original signature should verify with restored key
            let sig_original = identity.sign(&message);
            prop_assert!(restored.verify(&message, &sig_original));

            // Restored key's signature should verify with original
            let sig_restored = restored.sign(&message);
            prop_assert!(identity.verify(&message, &sig_restored));
        }

        /// Property: SHA-256 produces deterministic 32-byte output
        #[test]
        fn sha256_deterministic(data in prop::collection::vec(any::<u8>(), 0..2048)) {
            let hash1 = sha256(&data);
            let hash2 = sha256(&data);
            prop_assert_eq!(hash1, hash2);
            prop_assert_eq!(hash1.len(), 32);
        }

        /// Property: Different inputs produce different hashes (collision resistance)
        #[test]
        fn sha256_collision_resistant(
            data1 in prop::collection::vec(any::<u8>(), 1..512),
            data2 in prop::collection::vec(any::<u8>(), 1..512),
        ) {
            prop_assume!(data1 != data2);
            let hash1 = sha256(&data1);
            let hash2 = sha256(&data2);
            // Not guaranteed but extremely likely
            prop_assert_ne!(hash1, hash2);
        }

        /// Property: Empty or too-short private keys should error
        #[test]
        fn empty_private_key_rejected(bad_key in prop::collection::vec(any::<u8>(), 0..10)) {
            let result = EphemeralIdentity::from_private_key(&bad_key);
            prop_assert!(result.is_err());
        }

        /// Property: Too-long private keys should error
        #[test]
        fn long_private_key_rejected(bad_key in prop::collection::vec(any::<u8>(), 33..64)) {
            let result = EphemeralIdentity::from_private_key(&bad_key);
            prop_assert!(result.is_err());
        }
    }
}
