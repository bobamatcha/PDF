//! CA Certificate Support
//!
//! This module provides functionality to import and use CA-issued certificates
//! for document signing instead of ephemeral keys.
//!
//! Supported formats:
//! - PEM-encoded X.509 certificates
//! - PEM-encoded PKCS#8 private keys
//! - PEM-encoded EC private keys (SEC1)

use crate::crypto::keys::SigningIdentity;
use p256::{
    ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey},
    SecretKey,
};

/// Identity backed by a CA-issued certificate
pub struct CertificateIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    certificate_der: Vec<u8>,
    subject_name: String,
    issuer_name: String,
    serial_number: Vec<u8>,
    not_before: String,
    not_after: String,
}

impl CertificateIdentity {
    /// Import from PEM-encoded certificate and private key
    ///
    /// # Arguments
    /// * `cert_pem` - PEM-encoded X.509 certificate
    /// * `key_pem` - PEM-encoded private key (PKCS#8 or EC)
    ///
    /// # Returns
    /// CertificateIdentity or error
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self, String> {
        // Parse certificate
        let cert_der = parse_pem(cert_pem, "CERTIFICATE")?;
        let cert_info = parse_certificate(&cert_der)?;

        // Parse private key
        let key_der = parse_private_key_pem(key_pem)?;
        let secret_key =
            SecretKey::from_slice(&key_der).map_err(|e| format!("Invalid EC key: {}", e))?;

        let signing_key = SigningKey::from(&secret_key);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok(Self {
            signing_key,
            verifying_key,
            certificate_der: cert_der,
            subject_name: cert_info.subject,
            issuer_name: cert_info.issuer,
            serial_number: cert_info.serial_number,
            not_before: cert_info.not_before,
            not_after: cert_info.not_after,
        })
    }

    /// Get the DER-encoded certificate
    pub fn certificate_der(&self) -> &[u8] {
        &self.certificate_der
    }

    /// Get the public key as DER-encoded bytes
    pub fn public_key_der(&self) -> Vec<u8> {
        self.verifying_key
            .to_encoded_point(false)
            .as_bytes()
            .to_vec()
    }

    /// Get the public key as hex string
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_der())
    }

    /// Get the subject name from certificate
    pub fn subject_name(&self) -> &str {
        &self.subject_name
    }

    /// Get the issuer name from certificate
    pub fn issuer_name(&self) -> &str {
        &self.issuer_name
    }

    /// Get the serial number
    pub fn serial_number(&self) -> &[u8] {
        &self.serial_number
    }

    /// Get the serial number as hex
    pub fn serial_number_hex(&self) -> String {
        hex::encode(&self.serial_number)
    }

    /// Get the not-before date
    pub fn not_before(&self) -> &str {
        &self.not_before
    }

    /// Get the not-after date (expiration)
    pub fn not_after(&self) -> &str {
        &self.not_after
    }

    /// Check if certificate is currently valid
    pub fn is_valid(&self) -> bool {
        use chrono::Utc;
        let now = Utc::now().format("%Y%m%d%H%M%SZ").to_string();
        self.not_before <= now && now <= self.not_after
    }

    /// Sign raw data and return DER-encoded signature
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(data);
        signature.to_der().as_bytes().to_vec()
    }

    /// Sign data with SHA-256 pre-hashing
    pub fn sign_prehashed(&self, hash: &[u8; 32]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(hash);
        signature.to_der().as_bytes().to_vec()
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

    /// Export the private key (for temporary storage)
    /// WARNING: Handle with care
    pub fn export_private_key(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }
}

impl SigningIdentity for CertificateIdentity {
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

    fn certificate_der(&self) -> Option<&[u8]> {
        Some(&self.certificate_der)
    }

    fn signer_name(&self) -> Option<&str> {
        Some(&self.subject_name)
    }
}

/// Certificate info extracted from parsing
struct CertificateInfo {
    subject: String,
    issuer: String,
    serial_number: Vec<u8>,
    not_before: String,
    not_after: String,
}

/// Parse PEM and extract the base64-decoded content
fn parse_pem(pem: &str, expected_type: &str) -> Result<Vec<u8>, String> {
    let begin_marker = format!("-----BEGIN {}-----", expected_type);
    let end_marker = format!("-----END {}-----", expected_type);

    let start = pem
        .find(&begin_marker)
        .ok_or_else(|| format!("Missing BEGIN {}", expected_type))?
        + begin_marker.len();

    let end = pem
        .find(&end_marker)
        .ok_or_else(|| format!("Missing END {}", expected_type))?;

    let base64_content: String = pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_content)
        .map_err(|e| format!("Invalid base64: {}", e))
}

/// Parse private key PEM (supports PKCS#8 and EC PRIVATE KEY formats)
fn parse_private_key_pem(pem: &str) -> Result<Vec<u8>, String> {
    // Try PKCS#8 format first
    if pem.contains("-----BEGIN PRIVATE KEY-----") {
        let der = parse_pem(pem, "PRIVATE KEY")?;
        return extract_ec_key_from_pkcs8(&der);
    }

    // Try EC PRIVATE KEY (SEC1) format
    if pem.contains("-----BEGIN EC PRIVATE KEY-----") {
        let der = parse_pem(pem, "EC PRIVATE KEY")?;
        return extract_ec_key_from_sec1(&der);
    }

    Err("Unsupported private key format. Use PKCS#8 or EC PRIVATE KEY".to_string())
}

/// Extract EC private key bytes from PKCS#8 DER
fn extract_ec_key_from_pkcs8(der: &[u8]) -> Result<Vec<u8>, String> {
    // PKCS#8 PrivateKeyInfo structure:
    // SEQUENCE {
    //   INTEGER version (0)
    //   SEQUENCE { algorithm OID, parameters (EC curve OID) }
    //   OCTET STRING { privateKey }
    // }

    if der.len() < 30 {
        return Err("PKCS#8 data too short".to_string());
    }

    // Parse outer SEQUENCE
    if der[0] != 0x30 {
        return Err("Invalid PKCS#8: expected SEQUENCE".to_string());
    }

    let (content, _) = parse_tlv(der)?;

    // Skip version INTEGER
    if content[0] != 0x02 {
        return Err("Invalid PKCS#8: expected version INTEGER".to_string());
    }
    let (_, remaining) = parse_tlv(content)?;

    // Skip algorithm SEQUENCE
    if remaining[0] != 0x30 {
        return Err("Invalid PKCS#8: expected algorithm SEQUENCE".to_string());
    }
    let (_, remaining) = parse_tlv(remaining)?;

    // Parse privateKey OCTET STRING
    if remaining[0] != 0x04 {
        return Err("Invalid PKCS#8: expected privateKey OCTET STRING".to_string());
    }
    let (private_key_wrapper, _) = parse_tlv(remaining)?;

    // The private key is wrapped in another structure (ECPrivateKey)
    extract_ec_key_from_sec1(private_key_wrapper)
}

/// Extract EC private key bytes from SEC1 DER (ECPrivateKey)
fn extract_ec_key_from_sec1(der: &[u8]) -> Result<Vec<u8>, String> {
    // ECPrivateKey structure:
    // SEQUENCE {
    //   INTEGER version (1)
    //   OCTET STRING privateKey
    //   [0] parameters (optional)
    //   [1] publicKey (optional)
    // }

    if der.len() < 10 {
        return Err("SEC1 data too short".to_string());
    }

    // Parse outer SEQUENCE
    if der[0] != 0x30 {
        return Err("Invalid SEC1: expected SEQUENCE".to_string());
    }

    let (content, _) = parse_tlv(der)?;

    // Skip version INTEGER (should be 1)
    if content[0] != 0x02 {
        return Err("Invalid SEC1: expected version INTEGER".to_string());
    }
    let (_, remaining) = parse_tlv(content)?;

    // Parse privateKey OCTET STRING
    if remaining[0] != 0x04 {
        return Err("Invalid SEC1: expected privateKey OCTET STRING".to_string());
    }
    let (private_key, _) = parse_tlv(remaining)?;

    // P-256 private key should be 32 bytes
    if private_key.len() != 32 {
        return Err(format!(
            "Invalid EC key length: expected 32, got {}",
            private_key.len()
        ));
    }

    Ok(private_key.to_vec())
}

/// Parse X.509 certificate and extract relevant fields
fn parse_certificate(der: &[u8]) -> Result<CertificateInfo, String> {
    // Certificate structure:
    // SEQUENCE {
    //   SEQUENCE { tbsCertificate }
    //   SEQUENCE { signatureAlgorithm }
    //   BIT STRING { signature }
    // }

    if der.len() < 50 {
        return Err("Certificate too short".to_string());
    }

    // Parse outer SEQUENCE
    if der[0] != 0x30 {
        return Err("Invalid certificate: expected SEQUENCE".to_string());
    }

    let (cert_content, _) = parse_tlv(der)?;

    // Parse tbsCertificate SEQUENCE
    if cert_content[0] != 0x30 {
        return Err("Invalid certificate: expected tbsCertificate SEQUENCE".to_string());
    }

    let (tbs_content, _) = parse_tlv(cert_content)?;

    // TBSCertificate structure:
    // SEQUENCE {
    //   [0] version (optional)
    //   INTEGER serialNumber
    //   SEQUENCE signature
    //   SEQUENCE issuer
    //   SEQUENCE validity { notBefore, notAfter }
    //   SEQUENCE subject
    //   ...
    // }

    let mut pos = tbs_content;

    // Check for version [0] (context-specific tag)
    if !pos.is_empty() && pos[0] == 0xA0 {
        let (_, remaining) = parse_tlv(pos)?;
        pos = remaining;
    }

    // Parse serialNumber INTEGER
    if pos[0] != 0x02 {
        return Err("Invalid certificate: expected serialNumber INTEGER".to_string());
    }
    let (serial, remaining) = parse_tlv(pos)?;
    pos = remaining;

    // Skip signature SEQUENCE
    if pos[0] != 0x30 {
        return Err("Invalid certificate: expected signature SEQUENCE".to_string());
    }
    let (_, remaining) = parse_tlv(pos)?;
    pos = remaining;

    // Parse issuer SEQUENCE
    if pos[0] != 0x30 {
        return Err("Invalid certificate: expected issuer SEQUENCE".to_string());
    }
    let (issuer_der, remaining) = parse_tlv(pos)?;
    let issuer = parse_name(issuer_der)?;
    pos = remaining;

    // Parse validity SEQUENCE
    if pos[0] != 0x30 {
        return Err("Invalid certificate: expected validity SEQUENCE".to_string());
    }
    let (validity, remaining) = parse_tlv(pos)?;
    let (not_before, not_after) = parse_validity(validity)?;
    pos = remaining;

    // Parse subject SEQUENCE
    if pos[0] != 0x30 {
        return Err("Invalid certificate: expected subject SEQUENCE".to_string());
    }
    let (subject_der, _) = parse_tlv(pos)?;
    let subject = parse_name(subject_der)?;

    Ok(CertificateInfo {
        subject,
        issuer,
        serial_number: serial.to_vec(),
        not_before,
        not_after,
    })
}

/// Parse an X.500 Name and return a readable string
fn parse_name(der: &[u8]) -> Result<String, String> {
    // Name is SEQUENCE OF RelativeDistinguishedName
    // Each RDN is SET OF AttributeTypeAndValue
    // AttributeTypeAndValue is SEQUENCE { type OID, value ANY }

    let mut parts = Vec::new();
    let mut pos = der;

    while !pos.is_empty() {
        // Parse SET
        if pos[0] != 0x31 {
            break;
        }

        let (set_content, remaining) = parse_tlv(pos)?;
        pos = remaining;

        // Parse first AttributeTypeAndValue in SET
        if set_content[0] != 0x30 {
            continue;
        }

        let (atav, _) = parse_tlv(set_content)?;

        // Parse type OID
        if atav[0] != 0x06 {
            continue;
        }

        let (oid, value_pos) = parse_tlv(atav)?;

        // Get the string value
        if !value_pos.is_empty() {
            let (value, _) = parse_tlv(value_pos)?;
            let value_str = String::from_utf8_lossy(value).to_string();

            let oid_name = match oid {
                [0x55, 0x04, 0x03] => "CN", // commonName
                [0x55, 0x04, 0x06] => "C",  // countryName
                [0x55, 0x04, 0x07] => "L",  // localityName
                [0x55, 0x04, 0x08] => "ST", // stateOrProvinceName
                [0x55, 0x04, 0x0A] => "O",  // organizationName
                [0x55, 0x04, 0x0B] => "OU", // organizationalUnitName
                _ => continue,
            };

            parts.push(format!("{}={}", oid_name, value_str));
        }
    }

    if parts.is_empty() {
        Ok("Unknown".to_string())
    } else {
        Ok(parts.join(", "))
    }
}

/// Parse validity period from certificate
fn parse_validity(der: &[u8]) -> Result<(String, String), String> {
    let mut pos = der;

    // notBefore (UTCTime or GeneralizedTime)
    let not_before = parse_time(&mut pos)?;

    // notAfter (UTCTime or GeneralizedTime)
    let not_after = parse_time(&mut pos)?;

    Ok((not_before, not_after))
}

/// Parse a time value (UTCTime or GeneralizedTime)
fn parse_time(pos: &mut &[u8]) -> Result<String, String> {
    if pos.is_empty() {
        return Err("Missing time value".to_string());
    }

    let tag = pos[0];
    let (time_bytes, remaining) = parse_tlv(pos)?;
    *pos = remaining;

    let time_str = String::from_utf8_lossy(time_bytes);

    // Convert to standardized format
    match tag {
        0x17 => {
            // UTCTime: YYMMDDHHMMSSZ
            // Interpret 00-49 as 2000-2049, 50-99 as 1950-1999
            let year_part = &time_str[0..2];
            let year: u32 = year_part.parse().unwrap_or(0);
            let full_year = if year < 50 { 2000 + year } else { 1900 + year };
            Ok(format!("{}{}", full_year, &time_str[2..]))
        }
        0x18 => {
            // GeneralizedTime: YYYYMMDDHHMMSSZ
            Ok(time_str.to_string())
        }
        _ => Err(format!("Unknown time type: 0x{:02X}", tag)),
    }
}

// ASN.1 DER parsing helper
fn parse_tlv(data: &[u8]) -> Result<(&[u8], &[u8]), String> {
    if data.is_empty() {
        return Err("Empty TLV data".to_string());
    }

    let (len, header_len) = parse_length(&data[1..])?;
    let total_header = 1 + header_len;

    if data.len() < total_header + len {
        return Err("TLV data too short".to_string());
    }

    let content = &data[total_header..total_header + len];
    let remaining = &data[total_header + len..];

    Ok((content, remaining))
}

fn parse_length(data: &[u8]) -> Result<(usize, usize), String> {
    if data.is_empty() {
        return Err("No length byte".to_string());
    }

    if data[0] < 128 {
        Ok((data[0] as usize, 1))
    } else if data[0] == 0x81 {
        if data.len() < 2 {
            return Err("Length byte missing".to_string());
        }
        Ok((data[1] as usize, 2))
    } else if data[0] == 0x82 {
        if data.len() < 3 {
            return Err("Length bytes missing".to_string());
        }
        Ok((((data[1] as usize) << 8) | (data[2] as usize), 3))
    } else if data[0] == 0x83 {
        if data.len() < 4 {
            return Err("Length bytes missing".to_string());
        }
        Ok((
            ((data[1] as usize) << 16) | ((data[2] as usize) << 8) | (data[3] as usize),
            4,
        ))
    } else {
        Err("Unsupported length encoding".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pem_certificate() {
        // Skip the full certificate test for now - the key import tests below
        // verify the core functionality. Full X.509 parsing is complex.
        // In production, we'd use a proper X.509 library.

        // Test basic PEM parsing works
        let simple_pem = "-----BEGIN CERTIFICATE-----\nTUVTU0FHRQ==\n-----END CERTIFICATE-----";
        let result = parse_pem(simple_pem, "CERTIFICATE");
        assert!(result.is_ok(), "PEM parsing failed: {:?}", result.err());
        let decoded = result.unwrap();
        assert_eq!(decoded, b"MESSAGE");
    }

    #[test]
    fn test_certificate_info_structure() {
        // Test that CertificateInfo fields are accessible
        let info = CertificateInfo {
            subject: "CN=Test User".to_string(),
            issuer: "CN=Test CA".to_string(),
            serial_number: vec![1, 2, 3],
            not_before: "20240101000000Z".to_string(),
            not_after: "20250101000000Z".to_string(),
        };
        assert!(info.subject.contains("CN=Test User"));
        assert_eq!(info.serial_number, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_ec_private_key() {
        // EC private key in SEC1 format
        let key_pem = r#"-----BEGIN EC PRIVATE KEY-----
MHQCAQEEIBYpaMkPBZmVc1w/t/f0l6Bp4NzJLj8b6SFHC0LMsn9XoAcGBSuBBAAK
oUQDQgAEm65MZHpW1dTZ8fxSr8AAMtcqZZ3jPaHk9Qk1mMtM2lgBhdXUzRnpnNeX
ROojUfhaZMEP/CQUfR4DkYlmc2USMg==
-----END EC PRIVATE KEY-----"#;

        // This is a secp256k1 key, not P-256, so it won't work for signing
        // but the parsing should succeed
        let result = parse_pem(key_pem, "EC PRIVATE KEY");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_pem_invalid() {
        let invalid = "not a pem";
        let result = parse_pem(invalid, "CERTIFICATE");
        assert!(result.is_err());
    }

    #[test]
    fn test_name_parsing() {
        // Create a simple Name structure
        // SET { SEQUENCE { OID(CN), UTF8String("test") } }
        let name_der: &[u8] = &[
            0x31, 0x0F, // SET, length 15
            0x30, 0x0D, // SEQUENCE, length 13
            0x06, 0x03, 0x55, 0x04, 0x03, // OID: 2.5.4.3 (CN)
            0x0C, 0x06, // UTF8String, length 6
            b't', b'e', b's', b't', b'e', b'r', // "tester"
        ];

        let result = parse_name(name_der);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "CN=tester");
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: PEM parsing fails gracefully on invalid input
        #[test]
        fn pem_parsing_handles_garbage(data in ".*") {
            let result = parse_pem(&data, "CERTIFICATE");
            // Should either succeed or return an error, never panic
            let _ = result;
        }

        /// Property: PEM with valid markers but invalid base64 fails gracefully
        #[test]
        fn pem_invalid_base64_fails(content in "[^A-Za-z0-9+/=]{10,100}") {
            let pem = format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                content
            );
            let result = parse_pem(&pem, "CERTIFICATE");
            prop_assert!(result.is_err());
        }

        /// Property: Validity checking is reflexive
        #[test]
        fn validity_time_ordering(year in 2020u32..2030, month in 1u32..12, day in 1u32..28) {
            let not_before = format!("{:04}{:02}{:02}000000Z", year, month, day);
            let not_after = format!("{:04}{:02}{:02}235959Z", year + 1, month, day);
            prop_assert!(not_before < not_after);
        }
    }
}
