//! RFC 3161 Time-Stamp Authority (TSA) support for LTV
//!
//! This module provides functions to:
//! 1. Build TimeStampReq (timestamp request)
//! 2. Parse TimeStampResp (timestamp response)
//! 3. Extract TimeStampToken for embedding in CMS

use sha2::{Digest, Sha256};

/// OID for SHA-256: 2.16.840.1.101.3.4.2.1
const OID_SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];

/// OID for id-smime-aa-timeStampToken: 1.2.840.113549.1.9.16.2.14
pub const OID_TIMESTAMP_TOKEN: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x02, 0x0E,
];

/// OID for id-ct-TSTInfo: 1.2.840.113549.1.9.16.1.4
/// Used for TSA response parsing (future functionality)
#[allow(dead_code)]
const OID_TST_INFO: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x01, 0x04,
];

/// Build an RFC 3161 TimeStampReq for the given signature
///
/// # Arguments
/// * `signature` - The signature bytes to timestamp
///
/// # Returns
/// DER-encoded TimeStampReq
pub fn build_timestamp_request(signature: &[u8]) -> Vec<u8> {
    // Hash the signature
    let mut hasher = Sha256::new();
    hasher.update(signature);
    let hash: [u8; 32] = hasher.finalize().into();

    // Build MessageImprint
    let message_imprint = build_message_imprint(&hash);

    // Build TimeStampReq
    // TimeStampReq ::= SEQUENCE {
    //    version         INTEGER { v1(1) },
    //    messageImprint  MessageImprint,
    //    reqPolicy       TSAPolicyId OPTIONAL,
    //    nonce           INTEGER OPTIONAL,
    //    certReq         BOOLEAN DEFAULT FALSE,
    //    extensions      [0] IMPLICIT Extensions OPTIONAL
    // }
    let mut req_content = Vec::new();

    // version: 1
    req_content.extend(build_integer(&[1]));

    // messageImprint
    req_content.extend(message_imprint);

    // nonce (random 8 bytes) - helps prevent replay attacks
    let nonce = generate_nonce();
    req_content.extend(build_integer(&nonce));

    // certReq: true (we want the TSA certificate in the response)
    req_content.extend(build_boolean(true));

    build_sequence(&[&req_content])
}

/// Build MessageImprint structure
fn build_message_imprint(hash: &[u8]) -> Vec<u8> {
    // MessageImprint ::= SEQUENCE {
    //    hashAlgorithm   AlgorithmIdentifier,
    //    hashedMessage   OCTET STRING
    // }
    let alg_id = build_algorithm_identifier(OID_SHA256);
    let hashed_message = build_octet_string(hash);
    build_sequence(&[&alg_id, &hashed_message])
}

/// Parse a TimeStampResp and extract the TimeStampToken
///
/// # Arguments
/// * `response` - DER-encoded TimeStampResp
///
/// # Returns
/// The TimeStampToken bytes (to be embedded as unsigned attribute), or error
pub fn parse_timestamp_response(response: &[u8]) -> Result<Vec<u8>, String> {
    // TimeStampResp ::= SEQUENCE {
    //    status          PKIStatusInfo,
    //    timeStampToken  TimeStampToken OPTIONAL
    // }

    if response.is_empty() {
        return Err("Empty timestamp response".to_string());
    }

    // Parse outer SEQUENCE
    if response[0] != 0x30 {
        return Err("Invalid timestamp response: expected SEQUENCE".to_string());
    }

    let (content, _) = parse_tlv(response)?;

    // Parse PKIStatusInfo (first element)
    if content.is_empty() || content[0] != 0x30 {
        return Err("Invalid PKIStatusInfo".to_string());
    }

    let (status_info, remaining) = parse_tlv(content)?;

    // Check status (first element of PKIStatusInfo should be INTEGER)
    if status_info.is_empty() || status_info[0] != 0x02 {
        return Err("Invalid status in PKIStatusInfo".to_string());
    }

    let (status_value, _) = parse_tlv(status_info)?;
    if status_value.is_empty() || status_value[0] != 0 {
        // Status 0 = granted, anything else is an error
        let status_code = if status_value.is_empty() {
            255
        } else {
            status_value[0]
        };
        return Err(format!(
            "Timestamp request failed with status: {}",
            status_code
        ));
    }

    // Parse TimeStampToken (remaining content)
    if remaining.is_empty() {
        return Err("No TimeStampToken in response".to_string());
    }

    // The TimeStampToken is a ContentInfo containing SignedData
    // Return the entire token as-is for embedding
    Ok(remaining.to_vec())
}

/// Build the unsigned attributes containing the timestamp token
///
/// # Arguments
/// * `timestamp_token` - The TimeStampToken from TSA response
///
/// # Returns
/// DER-encoded unsigned attributes for SignerInfo
pub fn build_timestamp_unsigned_attr(timestamp_token: &[u8]) -> Vec<u8> {
    // Attribute ::= SEQUENCE {
    //    attrType   OID,
    //    attrValues SET OF AttributeValue
    // }
    let oid = build_oid(OID_TIMESTAMP_TOKEN);
    let value_set = build_set(timestamp_token);
    let attr = build_sequence(&[&oid, &value_set]);

    // Wrap in context-specific [1] for unsignedAttrs
    build_context_specific(1, &attr)
}

/// Verify basic structure of a timestamp token (optional validation)
pub fn validate_timestamp_token(token: &[u8]) -> Result<(), String> {
    if token.is_empty() {
        return Err("Empty timestamp token".to_string());
    }

    // Should start with SEQUENCE (ContentInfo)
    if token[0] != 0x30 {
        return Err("Invalid timestamp token: expected SEQUENCE".to_string());
    }

    // Basic length validation
    let (_, header_len) = parse_length(&token[1..])?;
    if token.len() < header_len + 2 {
        return Err("Timestamp token too short".to_string());
    }

    Ok(())
}

// === ASN.1 DER Encoding/Decoding Helpers ===

fn build_sequence(items: &[&[u8]]) -> Vec<u8> {
    let content: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    build_tlv(0x30, &content)
}

fn build_oid(oid_bytes: &[u8]) -> Vec<u8> {
    build_tlv(0x06, oid_bytes)
}

fn build_integer(value: &[u8]) -> Vec<u8> {
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

fn build_boolean(value: bool) -> Vec<u8> {
    build_tlv(0x01, &[if value { 0xFF } else { 0x00 }])
}

fn build_set(content: &[u8]) -> Vec<u8> {
    build_tlv(0x31, content)
}

fn build_context_specific(tag: u8, content: &[u8]) -> Vec<u8> {
    build_tlv(0xA0 | tag, content)
}

fn build_algorithm_identifier(oid: &[u8]) -> Vec<u8> {
    let oid_encoded = build_oid(oid);
    let null = vec![0x05, 0x00];
    build_sequence(&[&oid_encoded, &null])
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
    } else {
        Err("Unsupported length encoding".to_string())
    }
}

fn generate_nonce() -> Vec<u8> {
    // Generate 8 random bytes for nonce
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let nonce = timestamp.to_be_bytes();
    nonce[..8].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_timestamp_request() {
        let signature = b"test signature data";
        let request = build_timestamp_request(signature);

        // Should start with SEQUENCE
        assert_eq!(request[0], 0x30);

        // Should be non-empty
        assert!(request.len() > 10);
    }

    #[test]
    fn test_build_message_imprint() {
        let hash = [0u8; 32];
        let imprint = build_message_imprint(&hash);

        // Should be SEQUENCE containing AlgorithmIdentifier and OCTET STRING
        assert_eq!(imprint[0], 0x30);
    }

    #[test]
    fn test_build_unsigned_attr() {
        let token = vec![0x30, 0x03, 0x02, 0x01, 0x00]; // Minimal SEQUENCE
        let attr = build_timestamp_unsigned_attr(&token);

        // Should be context-specific [1]
        assert_eq!(attr[0], 0xA1);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Timestamp request is valid ASN.1 SEQUENCE
        #[test]
        fn timestamp_request_valid_structure(signature in prop::collection::vec(any::<u8>(), 1..1000)) {
            let request = build_timestamp_request(&signature);

            // Should start with SEQUENCE tag
            prop_assert_eq!(request[0], 0x30);

            // Should have valid length
            let len = if request[1] < 128 {
                request[1] as usize
            } else if request[1] == 0x81 {
                request[2] as usize
            } else {
                ((request[2] as usize) << 8) | (request[3] as usize)
            };

            prop_assert!(len > 0);
        }

        /// Property: Message imprint contains hash of input
        #[test]
        fn message_imprint_contains_hash(data in prop::collection::vec(any::<u8>(), 0..500)) {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash: [u8; 32] = hasher.finalize().into();

            let imprint = build_message_imprint(&hash);

            // Should contain the hash bytes somewhere in the output
            let hash_found = imprint.windows(32).any(|w| w == hash);
            prop_assert!(hash_found);
        }

        /// Property: Unsigned attribute has correct structure
        #[test]
        fn unsigned_attr_structure(token in prop::collection::vec(any::<u8>(), 1..200)) {
            let attr = build_timestamp_unsigned_attr(&token);

            // Should start with context-specific [1]
            prop_assert_eq!(attr[0], 0xA1);

            // Should be long enough to contain the token
            prop_assert!(attr.len() > token.len());
        }
    }
}
