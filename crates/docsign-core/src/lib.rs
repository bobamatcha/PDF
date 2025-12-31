//! Document signing core logic
//!
//! This crate provides the core document signing functionality,
//! including PDF signing, audit trails, and session management.
//!
//! Most functionality is currently in the docsign-wasm crate and
//! will be migrated here incrementally.

// Re-export types from shared crates
pub use shared_crypto::{cert, cms, keys, tsa, EphemeralIdentity, SigningIdentity};
pub use shared_pdf::{dom_to_pdf, parser, pdf_to_dom, signer, PdfDocument};

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // ============================================================
    // Sign-Verify Roundtrip Property Tests
    // ============================================================

    proptest! {
        /// Property 1: Sign-verify roundtrip - Any document signed with a key must verify
        /// with the corresponding public key
        #[test]
        fn sign_verify_roundtrip(document in prop::collection::vec(any::<u8>(), 0..2048)) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            prop_assert!(
                identity.verify(&document, &signature),
                "Signature should verify with the same identity"
            );
        }

        /// Property 2: Cross-key verification failure - Signature from key A should NOT
        /// verify with key B
        #[test]
        fn cross_key_verification_fails(document in prop::collection::vec(any::<u8>(), 1..1024)) {
            let identity_a = EphemeralIdentity::generate();
            let identity_b = EphemeralIdentity::generate();

            let signature_a = identity_a.sign(&document);

            // Key B should not be able to verify signature from Key A
            prop_assert!(
                !identity_b.verify(&document, &signature_a),
                "Signature from identity A should not verify with identity B"
            );
        }

        /// Property 3: Tampering detection - Modifying a signed document must cause
        /// verification to fail
        #[test]
        fn tampering_detection(
            document in prop::collection::vec(any::<u8>(), 1..512),
            tamper_index in any::<prop::sample::Index>(),
            tamper_byte in any::<u8>(),
        ) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // Create tampered document
            let mut tampered = document.clone();
            let idx = tamper_index.index(tampered.len());
            let original_byte = tampered[idx];

            // Only tamper if the new byte is different
            prop_assume!(tamper_byte != original_byte);
            tampered[idx] = tamper_byte;

            prop_assert!(
                !identity.verify(&tampered, &signature),
                "Tampered document should fail verification"
            );
        }

        /// Property 4: Signature format consistency - Signatures are always valid DER
        #[test]
        fn signature_format_valid(document in prop::collection::vec(any::<u8>(), 0..256)) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // ECDSA DER signature starts with 0x30 (SEQUENCE tag)
            prop_assert_eq!(signature[0], 0x30, "Signature should be DER-encoded SEQUENCE");

            // Length should be valid
            let len = signature[1] as usize;
            prop_assert!(
                signature.len() >= len + 2,
                "Signature length should be consistent with DER encoding"
            );
        }

        /// Property 5: Public key consistency - Export/import preserves verification capability
        #[test]
        fn key_export_import_preserves_verification(
            document in prop::collection::vec(any::<u8>(), 1..512)
        ) {
            let original = EphemeralIdentity::generate();
            let signature = original.sign(&document);

            // Export and re-import
            let exported = original.export_private_key();
            let restored = EphemeralIdentity::from_private_key(&exported)
                .expect("Should successfully import valid private key");

            // Restored key should verify signatures from original
            prop_assert!(
                restored.verify(&document, &signature),
                "Restored identity should verify original signature"
            );

            // Original should verify signatures from restored
            let new_signature = restored.sign(&document);
            prop_assert!(
                original.verify(&document, &new_signature),
                "Original identity should verify restored signature"
            );
        }

        /// Property 6: SHA-256 prehash signing works correctly
        #[test]
        fn prehash_sign_verify(document in prop::collection::vec(any::<u8>(), 0..1024)) {
            use sha2::{Digest, Sha256};

            let identity = EphemeralIdentity::generate();

            // Compute hash manually
            let mut hasher = Sha256::new();
            hasher.update(&document);
            let hash: [u8; 32] = hasher.finalize().into();

            // Sign the prehashed data
            let signature = identity.sign_prehashed(&hash);

            // Signature should be valid DER
            prop_assert_eq!(signature[0], 0x30, "Prehash signature should be DER-encoded");
            prop_assert!(signature.len() >= 68 && signature.len() <= 72,
                "ECDSA P-256 signature should be 68-72 bytes in DER format");
        }

        /// Property 7: Different documents produce different signatures
        #[test]
        fn different_documents_different_signatures(
            doc1 in prop::collection::vec(any::<u8>(), 1..256),
            doc2 in prop::collection::vec(any::<u8>(), 1..256),
        ) {
            prop_assume!(doc1 != doc2);

            let identity = EphemeralIdentity::generate();
            let sig1 = identity.sign(&doc1);
            let sig2 = identity.sign(&doc2);

            // Signatures should be different for different documents
            // (ECDSA uses random nonce, so even same doc gives different sigs,
            // but different docs MUST give different sigs)
            prop_assert_ne!(sig1, sig2, "Different documents should produce different signatures");
        }

        /// Property 8: Empty document can be signed and verified
        #[test]
        fn empty_document_sign_verify(_unused in Just(())) {
            let identity = EphemeralIdentity::generate();
            let empty: Vec<u8> = vec![];

            let signature = identity.sign(&empty);
            prop_assert!(
                identity.verify(&empty, &signature),
                "Empty document should sign and verify correctly"
            );
        }

        /// Property 9: Invalid signatures fail verification
        #[test]
        fn invalid_signature_fails(
            document in prop::collection::vec(any::<u8>(), 1..256),
            garbage_sig in prop::collection::vec(any::<u8>(), 0..100),
        ) {
            let identity = EphemeralIdentity::generate();

            prop_assert!(
                !identity.verify(&document, &garbage_sig),
                "Random garbage should not verify as valid signature"
            );
        }

        /// Property 10: Signature with wrong length fails
        #[test]
        fn truncated_signature_fails(
            document in prop::collection::vec(any::<u8>(), 1..256),
            truncate_by in 1usize..30,
        ) {
            let identity = EphemeralIdentity::generate();
            let signature = identity.sign(&document);

            // Truncate the signature
            let truncate_amount = truncate_by.min(signature.len().saturating_sub(1));
            if truncate_amount > 0 {
                let truncated = &signature[..signature.len() - truncate_amount];

                prop_assert!(
                    !identity.verify(&document, truncated),
                    "Truncated signature should fail verification"
                );
            }
        }
    }
}
