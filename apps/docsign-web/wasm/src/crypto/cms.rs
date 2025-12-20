//! CMS (Cryptographic Message Syntax) construction for PAdES
//!
//! This implementation creates a PAdES-B compliant PKCS#7 SignedData
//! structure suitable for embedding in PDFs with the following attributes:
//! - content-type
//! - signing-time
//! - message-digest
//! - signing-certificate-v2 (ESS, required for PAdES-B)

use sha2::{Digest, Sha256};

/// OID for SHA-256: 2.16.840.1.101.3.4.2.1
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];

/// OID for ECDSA with SHA-256: 1.2.840.10045.4.3.2
const OID_ECDSA_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];

/// OID for id-data (PKCS#7): 1.2.840.113549.1.7.1
const OID_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];

/// OID for id-signedData (PKCS#7): 1.2.840.113549.1.7.2
const OID_SIGNED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];

/// OID for content-type attribute: 1.2.840.113549.1.9.3
const OID_CONTENT_TYPE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x03];

/// OID for message-digest attribute: 1.2.840.113549.1.9.4
const OID_MESSAGE_DIGEST: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04];

/// OID for signing-time attribute: 1.2.840.113549.1.9.5
const OID_SIGNING_TIME: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05];

/// OID for id-aa-signingCertificateV2: 1.2.840.113549.1.9.16.2.47
/// Required for PAdES-B compliance
const OID_SIGNING_CERTIFICATE_V2: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x2F,
];

/// Build a PAdES-B compliant CMS SignedData structure for PDF embedding
///
/// # Arguments
/// * `document_hash` - SHA-256 hash of the PDF byte range being signed
/// * `signature` - DER-encoded ECDSA signature
/// * `public_key` - SEC1-encoded public key
/// * `signer_name` - Common name for the signer
/// * `signing_time` - UTC time string (YYYYMMDDHHMMSSZ format)
pub fn build_signed_data(
    document_hash: &[u8],
    signature: &[u8],
    public_key: &[u8],
    signer_name: &str,
    signing_time: &str,
) -> Vec<u8> {
    // Build certificate first (needed for signing-certificate-v2 attribute)
    let certificate = build_self_signed_cert(public_key, signer_name);

    // Build authenticated attributes (including PAdES-B required signing-certificate-v2)
    let auth_attrs = build_authenticated_attributes(document_hash, signing_time, &certificate);

    // Build SignerInfo
    let signer_info = build_signer_info(&auth_attrs, signature, signer_name);

    // Build SignedData
    let signed_data = build_signed_data_content(&certificate, &signer_info);

    // Wrap in ContentInfo
    build_content_info(&signed_data)
}

/// Build authenticated attributes for PAdES-B compliance
/// Includes: content-type, signing-time, message-digest, signing-certificate-v2
fn build_authenticated_attributes(
    document_hash: &[u8],
    signing_time: &str,
    certificate: &[u8],
) -> Vec<u8> {
    let mut attrs = Vec::new();

    // Content-type attribute (required)
    let content_type_attr = build_attribute(OID_CONTENT_TYPE, &build_oid(OID_DATA));
    attrs.extend(content_type_attr);

    // Signing-time attribute (required for PAdES)
    let time_value = build_utc_time(signing_time);
    let signing_time_attr = build_attribute(OID_SIGNING_TIME, &time_value);
    attrs.extend(signing_time_attr);

    // Message-digest attribute (required)
    let digest_value = build_octet_string(document_hash);
    let message_digest_attr = build_attribute(OID_MESSAGE_DIGEST, &digest_value);
    attrs.extend(message_digest_attr);

    // Signing-certificate-v2 attribute (required for PAdES-B)
    let signing_cert_attr = build_signing_certificate_v2(certificate);
    attrs.extend(signing_cert_attr);

    // Wrap as SET
    build_set(&attrs)
}

/// Build the ESS signing-certificate-v2 attribute for PAdES-B compliance
/// SigningCertificateV2 ::= SEQUENCE {
///     certs SEQUENCE OF ESSCertIDv2,
///     policies SEQUENCE OF PolicyInformation OPTIONAL
/// }
/// ESSCertIDv2 ::= SEQUENCE {
///     hashAlgorithm AlgorithmIdentifier DEFAULT {algorithm id-sha256},
///     certHash Hash,
///     issuerSerial IssuerSerial OPTIONAL
/// }
fn build_signing_certificate_v2(certificate: &[u8]) -> Vec<u8> {
    // Hash the certificate with SHA-256
    let mut hasher = Sha256::new();
    hasher.update(certificate);
    let cert_hash: [u8; 32] = hasher.finalize().into();

    // Build ESSCertIDv2
    // When using SHA-256 (the default), hashAlgorithm can be omitted
    // For simplicity, we include it explicitly
    let hash_alg = build_algorithm_identifier(OID_SHA256);
    let hash_value = build_octet_string(&cert_hash);
    let ess_cert_id = build_sequence(&[&hash_alg, &hash_value]);

    // Wrap in SEQUENCE OF ESSCertIDv2
    let certs = build_sequence(&[&ess_cert_id]);

    // Build SigningCertificateV2
    let signing_cert = build_sequence(&[&certs]);

    // Build as attribute
    build_attribute(OID_SIGNING_CERTIFICATE_V2, &signing_cert)
}

/// Build a single attribute (SEQUENCE of OID and SET of values)
fn build_attribute(oid: &[u8], value: &[u8]) -> Vec<u8> {
    let oid_encoded = build_oid(oid);
    let value_set = build_set(value);
    build_sequence(&[&oid_encoded, &value_set])
}

/// Build SignerInfo structure
fn build_signer_info(auth_attrs: &[u8], signature: &[u8], _signer_name: &str) -> Vec<u8> {
    let mut content = Vec::new();

    // Version (1 for issuerAndSerialNumber)
    content.extend(build_integer(&[1]));

    // IssuerAndSerialNumber (simplified)
    let issuer_serial = build_issuer_and_serial();
    content.extend(issuer_serial);

    // DigestAlgorithm (SHA-256)
    let digest_alg = build_algorithm_identifier(OID_SHA256);
    content.extend(digest_alg);

    // Authenticated attributes (context-specific tag [0])
    let auth_attrs_tagged = build_context_specific(0, auth_attrs);
    content.extend(auth_attrs_tagged);

    // Signature algorithm (ECDSA-SHA256)
    let sig_alg = build_algorithm_identifier(OID_ECDSA_SHA256);
    content.extend(sig_alg);

    // Signature value
    content.extend(build_octet_string(signature));

    build_sequence(&[&content])
}

/// Build a self-signed certificate placeholder
fn build_self_signed_cert(public_key: &[u8], signer_name: &str) -> Vec<u8> {
    let mut tbs = Vec::new();

    // Version (v3 = 2)
    tbs.extend(build_context_specific(0, &build_integer(&[2])));

    // Serial number
    tbs.extend(build_integer(&[1]));

    // Signature algorithm
    tbs.extend(build_algorithm_identifier(OID_ECDSA_SHA256));

    // Issuer (CN=signer_name)
    let issuer = build_name(signer_name);
    tbs.extend(issuer);

    // Validity (placeholder - 1 year)
    tbs.extend(build_validity());

    // Subject (same as issuer for self-signed)
    tbs.extend(build_name(signer_name));

    // SubjectPublicKeyInfo
    tbs.extend(build_subject_public_key_info(public_key));

    let tbs_cert = build_sequence(&[&tbs]);

    // Full certificate (TBS + algorithm + signature placeholder)
    let mut cert = Vec::new();
    cert.extend(&tbs_cert);
    cert.extend(build_algorithm_identifier(OID_ECDSA_SHA256));
    cert.extend(build_bit_string(&[0; 64])); // Placeholder signature

    build_sequence(&[&cert])
}

/// Build SignedData content
fn build_signed_data_content(certificate: &[u8], signer_info: &[u8]) -> Vec<u8> {
    let mut content = Vec::new();

    // Version (1)
    content.extend(build_integer(&[1]));

    // DigestAlgorithms (SET of SHA-256)
    let digest_alg = build_algorithm_identifier(OID_SHA256);
    content.extend(build_set(&digest_alg));

    // EncapsulatedContentInfo (empty for detached)
    let encap_content = build_sequence(&[&build_oid(OID_DATA)]);
    content.extend(encap_content);

    // Certificates [0] IMPLICIT
    content.extend(build_context_specific(0, certificate));

    // SignerInfos (SET of SignerInfo)
    content.extend(build_set(signer_info));

    build_sequence(&[&content])
}

/// Build ContentInfo wrapper
fn build_content_info(signed_data: &[u8]) -> Vec<u8> {
    let oid = build_oid(OID_SIGNED_DATA);
    let content = build_context_specific(0, signed_data);
    build_sequence(&[&oid, &content])
}

// === ASN.1 DER Encoding Helpers ===

fn build_sequence(items: &[&[u8]]) -> Vec<u8> {
    let content: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    build_tlv(0x30, &content)
}

fn build_set(content: &[u8]) -> Vec<u8> {
    build_tlv(0x31, content)
}

fn build_oid(oid_bytes: &[u8]) -> Vec<u8> {
    build_tlv(0x06, oid_bytes)
}

fn build_integer(value: &[u8]) -> Vec<u8> {
    // Ensure proper encoding (add leading zero if high bit set)
    if !value.is_empty() && value[0] & 0x80 != 0 {
        let mut padded = vec![0];
        padded.extend(value);
        build_tlv(0x02, &padded)
    } else {
        build_tlv(0x02, value)
    }
}

fn build_octet_string(content: &[u8]) -> Vec<u8> {
    build_tlv(0x04, content)
}

fn build_bit_string(content: &[u8]) -> Vec<u8> {
    let mut bs = vec![0]; // No unused bits
    bs.extend(content);
    build_tlv(0x03, &bs)
}

fn build_utf8_string(s: &str) -> Vec<u8> {
    build_tlv(0x0C, s.as_bytes())
}

fn build_utc_time(time: &str) -> Vec<u8> {
    // Convert to YYMMDDHHMMSSZ format if needed
    let formatted = if time.len() > 13 {
        &time[2..15] // Strip century and timezone
    } else {
        time
    };
    build_tlv(0x17, formatted.as_bytes())
}

fn build_context_specific(tag: u8, content: &[u8]) -> Vec<u8> {
    build_tlv(0xA0 | tag, content)
}

fn build_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
    let len = content.len();

    if len < 128 {
        result.push(len as u8);
    } else if len < 256 {
        result.push(0x81);
        result.push(len as u8);
    } else {
        result.push(0x82);
        result.push((len >> 8) as u8);
        result.push(len as u8);
    }

    result.extend(content);
    result
}

fn build_algorithm_identifier(oid: &[u8]) -> Vec<u8> {
    let oid_encoded = build_oid(oid);
    let null = vec![0x05, 0x00]; // NULL parameters
    build_sequence(&[&oid_encoded, &null])
}

fn build_name(cn: &str) -> Vec<u8> {
    // RDN: SET { SEQUENCE { OID (CN), UTF8String } }
    let cn_oid = build_oid(&[0x55, 0x04, 0x03]); // 2.5.4.3 = CN
    let cn_value = build_utf8_string(cn);
    let attr = build_sequence(&[&cn_oid, &cn_value]);
    let rdn = build_set(&attr);
    build_sequence(&[&rdn])
}

fn build_validity() -> Vec<u8> {
    let not_before = build_utc_time("240101000000Z");
    let not_after = build_utc_time("250101000000Z");
    build_sequence(&[&not_before, &not_after])
}

fn build_issuer_and_serial() -> Vec<u8> {
    let issuer = build_name("DocSign Ephemeral");
    let serial = build_integer(&[1]);
    build_sequence(&[&issuer, &serial])
}

fn build_subject_public_key_info(public_key: &[u8]) -> Vec<u8> {
    // OID for EC public key: 1.2.840.10045.2.1
    let ec_oid = build_oid(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01]);
    // OID for P-256: 1.2.840.10045.3.1.7
    let p256_oid = build_oid(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07]);
    let alg = build_sequence(&[&ec_oid, &p256_oid]);
    let pk_bits = build_bit_string(public_key);
    build_sequence(&[&alg, &pk_bits])
}

/// Compute SHA-256 of data for signing authenticated attributes
pub fn hash_authenticated_attrs(auth_attrs: &[u8]) -> [u8; 32] {
    // When signing, authenticated attributes are hashed as a SET (tag 0x31)
    let mut hasher = Sha256::new();
    hasher.update(auth_attrs);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_integer() {
        let int = build_integer(&[0x01]);
        assert_eq!(int, vec![0x02, 0x01, 0x01]);

        // High bit set - needs padding
        let int_padded = build_integer(&[0x80]);
        assert_eq!(int_padded, vec![0x02, 0x02, 0x00, 0x80]);
    }

    #[test]
    fn test_build_sequence() {
        let seq = build_sequence(&[&[0x02, 0x01, 0x01], &[0x02, 0x01, 0x02]]);
        assert_eq!(seq[0], 0x30); // SEQUENCE tag
        assert_eq!(seq[1], 0x06); // Length
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: TLV encoding produces valid length-prefixed output
        #[test]
        fn tlv_length_correct(content in prop::collection::vec(any::<u8>(), 0..500)) {
            let tlv = build_tlv(0x04, &content); // OCTET STRING

            prop_assert_eq!(tlv[0], 0x04); // Tag

            // Parse length
            let (reported_len, header_len) = if tlv[1] < 128 {
                (tlv[1] as usize, 2)
            } else if tlv[1] == 0x81 {
                (tlv[2] as usize, 3)
            } else {
                ((tlv[2] as usize) << 8 | tlv[3] as usize, 4)
            };

            prop_assert_eq!(reported_len, content.len());
            prop_assert_eq!(tlv.len(), header_len + content.len());
        }

        /// Property: Integer encoding handles high-bit padding
        #[test]
        fn integer_high_bit_handled(byte in any::<u8>()) {
            let int = build_integer(&[byte]);

            prop_assert_eq!(int[0], 0x02); // INTEGER tag

            if byte & 0x80 != 0 {
                // Should have padding byte
                prop_assert_eq!(int[1], 2); // Length = 2
                prop_assert_eq!(int[2], 0); // Padding
                prop_assert_eq!(int[3], byte); // Original value
            } else {
                prop_assert_eq!(int[1], 1); // Length = 1
                prop_assert_eq!(int[2], byte); // Original value
            }
        }

        /// Property: Sequence wraps content correctly
        #[test]
        fn sequence_structure(
            item1 in prop::collection::vec(any::<u8>(), 1..50),
            item2 in prop::collection::vec(any::<u8>(), 1..50),
        ) {
            let seq = build_sequence(&[&item1, &item2]);

            prop_assert_eq!(seq[0], 0x30); // SEQUENCE tag

            // Content should be concatenation of items
            let expected_len = item1.len() + item2.len();

            // Parse length
            let (reported_len, header_len) = if seq[1] < 128 {
                (seq[1] as usize, 2)
            } else if seq[1] == 0x81 {
                (seq[2] as usize, 3)
            } else {
                ((seq[2] as usize) << 8 | seq[3] as usize, 4)
            };

            prop_assert_eq!(reported_len, expected_len);
            prop_assert_eq!(seq.len(), header_len + expected_len);
        }

        /// Property: OID encoding preserves content
        #[test]
        fn oid_preserves_bytes(oid_bytes in prop::collection::vec(any::<u8>(), 1..20)) {
            let oid = build_oid(&oid_bytes);

            prop_assert_eq!(oid[0], 0x06); // OID tag
            prop_assert_eq!(oid[1], oid_bytes.len() as u8); // Length
            prop_assert_eq!(&oid[2..], &oid_bytes[..]);
        }

        /// Property: Octet string encoding is correct
        #[test]
        fn octet_string_correct(content in prop::collection::vec(any::<u8>(), 0..200)) {
            let os = build_octet_string(&content);

            prop_assert_eq!(os[0], 0x04); // OCTET STRING tag

            // Extract content from TLV
            let (len, header_len) = if os[1] < 128 {
                (os[1] as usize, 2)
            } else if os[1] == 0x81 {
                (os[2] as usize, 3)
            } else {
                ((os[2] as usize) << 8 | os[3] as usize, 4)
            };

            prop_assert_eq!(len, content.len());
            prop_assert_eq!(&os[header_len..], &content[..]);
        }

        /// Property: build_signed_data produces valid ASN.1 structure
        #[test]
        fn signed_data_valid_structure(
            doc_hash in prop::collection::vec(any::<u8>(), 32..=32),
            signature in prop::collection::vec(any::<u8>(), 64..=72),
            public_key in prop::collection::vec(any::<u8>(), 65..=65),
        ) {
            let cms = build_signed_data(
                &doc_hash,
                &signature,
                &public_key,
                "Test Signer",
                "20240101120000Z",
            );

            // Should start with SEQUENCE tag (ContentInfo)
            prop_assert_eq!(cms[0], 0x30);

            // Length should be valid
            let len = if cms[1] < 128 {
                cms[1] as usize
            } else if cms[1] == 0x81 {
                cms[2] as usize
            } else {
                (cms[2] as usize) << 8 | cms[3] as usize
            };

            prop_assert!(len > 0);
            prop_assert!(cms.len() > len); // Total > content length (has header)
        }

        /// Property: UTF8 string encoding is correct
        #[test]
        fn utf8_string_correct(s in "[a-zA-Z0-9 ]{1,50}") {
            let utf8 = build_utf8_string(&s);

            prop_assert_eq!(utf8[0], 0x0C); // UTF8String tag
            prop_assert_eq!(utf8[1], s.len() as u8); // Length
            prop_assert_eq!(&utf8[2..], s.as_bytes());
        }
    }
}
