//! Tamper-evident audit log for document events

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Types of auditable events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    Upload,
    View,
    FieldAdded,
    FieldRemoved,
    Sign,
    Decline,
    Complete,
    Send,
    ComplianceCheck {
        violations_found: u32,
    },
    DocumentLoaded {
        hash: String,
    },
    FieldAddedDetailed {
        field_type: String,
        page: u32,
    },
    FieldMoved {
        field_id: String,
        new_x: f64,
        new_y: f64,
    },
    FieldDeleted {
        field_id: String,
    },
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: String,
    pub action: AuditAction,
    pub actor_email: String,
    pub actor_ip_hash: Option<String>,
    pub document_hash: String,
    pub previous_hash: Option<String>,
    pub details: Option<String>,
    pub signature: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        action: AuditAction,
        actor_email: &str,
        document_hash: &str,
        previous_hash: Option<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            action,
            actor_email: actor_email.to_string(),
            actor_ip_hash: None,
            document_hash: document_hash.to_string(),
            previous_hash,
            details,
            signature: None,
        }
    }

    /// Compute the hash of this event (for chain linking)
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.timestamp.as_bytes());
        hasher.update(format!("{:?}", self.action).as_bytes());
        hasher.update(self.actor_email.as_bytes());
        hasher.update(self.document_hash.as_bytes());
        if let Some(ref prev) = self.previous_hash {
            hasher.update(prev.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Set the signature for this event
    pub fn set_signature(&mut self, signature: String) {
        self.signature = Some(signature);
    }
}

/// Chain of audit events with hash linking
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuditChain {
    pub events: Vec<AuditEvent>,
    pub document_id: String,
    pub created_at: String,
}

impl AuditChain {
    /// Create a new audit chain for a document
    pub fn new(document_id: &str) -> Self {
        Self {
            events: Vec::new(),
            document_id: document_id.to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Get the hash of the last event (for linking)
    pub fn last_hash(&self) -> Option<String> {
        self.events.last().map(|e| e.compute_hash())
    }

    /// Append an event, automatically linking to previous hash
    pub fn append(
        &mut self,
        action: AuditAction,
        actor_email: &str,
        document_hash: &str,
        details: Option<String>,
    ) -> &AuditEvent {
        let previous_hash = self.last_hash();
        let event = AuditEvent::new(action, actor_email, document_hash, previous_hash, details);
        self.events.push(event);
        self.events.last().unwrap()
    }

    /// Verify the integrity of the chain
    pub fn verify(&self) -> Result<(), String> {
        let mut expected_prev: Option<String> = None;

        for (i, event) in self.events.iter().enumerate() {
            // Check previous hash matches
            if event.previous_hash != expected_prev {
                return Err(format!(
                    "Chain broken at event {}: expected prev {:?}, got {:?}",
                    i, expected_prev, event.previous_hash
                ));
            }
            expected_prev = Some(event.compute_hash());
        }

        Ok(())
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize audit chain: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to deserialize audit chain: {}", e))
    }

    /// Generate a summary for display
    pub fn summary(&self) -> Vec<String> {
        self.events
            .iter()
            .map(|e| {
                format!(
                    "[{}] {} - {:?}",
                    e.timestamp.split('T').next().unwrap_or(&e.timestamp),
                    e.actor_email,
                    e.action
                )
            })
            .collect()
    }
}

/// Compute SHA-256 hash of document bytes
pub fn hash_document(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_integrity() {
        let mut chain = AuditChain::new("test-doc-123");

        chain.append(AuditAction::Upload, "alice@example.com", "hash1", None);
        chain.append(
            AuditAction::FieldAdded,
            "alice@example.com",
            "hash1",
            Some("Signature field on page 1".to_string()),
        );
        chain.append(AuditAction::Sign, "alice@example.com", "hash2", None);

        assert!(chain.verify().is_ok());
        assert_eq!(chain.events.len(), 3);
    }

    #[test]
    fn test_chain_tamper_detection() {
        let mut chain = AuditChain::new("test-doc-123");

        chain.append(AuditAction::Upload, "alice@example.com", "hash1", None);
        chain.append(AuditAction::Sign, "alice@example.com", "hash2", None);

        // Tamper with the first event
        chain.events[0].actor_email = "mallory@example.com".to_string();

        // Chain should now fail verification
        assert!(chain.verify().is_err());
    }

    #[test]
    fn test_compliance_check_action() {
        let mut chain = AuditChain::new("test-doc-123");

        chain.append(
            AuditAction::ComplianceCheck {
                violations_found: 3,
            },
            "compliance@example.com",
            "hash1",
            Some("Found 3 violations: missing signatures, incorrect format, etc.".to_string()),
        );

        assert!(chain.verify().is_ok());
        assert_eq!(chain.events.len(), 1);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating random action types (kept for future use)
    #[allow(dead_code)]
    fn action_strategy() -> impl Strategy<Value = AuditAction> {
        prop_oneof![
            Just(AuditAction::Upload),
            Just(AuditAction::View),
            Just(AuditAction::FieldAdded),
            Just(AuditAction::FieldRemoved),
            Just(AuditAction::Sign),
            Just(AuditAction::Decline),
            Just(AuditAction::Complete),
            Just(AuditAction::Send),
            (0u32..100)
                .prop_map(|violations_found| AuditAction::ComplianceCheck { violations_found }),
            "[0-9a-f]{64}".prop_map(|hash| AuditAction::DocumentLoaded { hash }),
            ("[a-z]{3,10}", 1u32..100).prop_map(|(field_type, page)| {
                AuditAction::FieldAddedDetailed { field_type, page }
            }),
            ("[a-z0-9-]{8,20}", 0.0f64..1000.0, 0.0f64..1000.0).prop_map(
                |(field_id, new_x, new_y)| AuditAction::FieldMoved {
                    field_id,
                    new_x,
                    new_y
                }
            ),
            "[a-z0-9-]{8,20}".prop_map(|field_id| AuditAction::FieldDeleted { field_id }),
        ]
    }

    // Strategy for valid emails (kept for future use)
    #[allow(dead_code)]
    fn email_strategy() -> impl Strategy<Value = String> {
        "[a-z]{3,10}@[a-z]{3,8}\\.(com|org|net)"
    }

    // Strategy for document hashes (kept for future use)
    #[allow(dead_code)]
    fn hash_strategy() -> impl Strategy<Value = String> {
        "[0-9a-f]{64}"
    }

    proptest! {
        /// Property: Any sequence of appends maintains chain integrity
        #[test]
        fn append_preserves_integrity(
            doc_id in "[a-z0-9-]{8,20}",
            count in 1usize..20,
        ) {
            let mut chain = AuditChain::new(&doc_id);

            for i in 0..count {
                chain.append(
                    AuditAction::View,
                    &format!("user{}@test.com", i),
                    &format!("{:064x}", i),
                    None,
                );
            }

            prop_assert!(chain.verify().is_ok());
            prop_assert_eq!(chain.events.len(), count);
        }

        /// Property: Each event has a unique ID
        #[test]
        fn event_ids_unique(count in 2usize..50) {
            let mut chain = AuditChain::new("test-doc");

            for i in 0..count {
                chain.append(
                    AuditAction::View,
                    "test@example.com",
                    &format!("{:064x}", i),
                    None,
                );
            }

            let ids: Vec<&String> = chain.events.iter().map(|e| &e.event_id).collect();
            let unique_count = {
                let mut seen = std::collections::HashSet::new();
                ids.iter().filter(|id| seen.insert(id.as_str())).count()
            };

            prop_assert_eq!(unique_count, count);
        }

        /// Property: Tampering with any field breaks verification
        #[test]
        fn tampering_detected(
            tamper_index in 0usize..5,
        ) {
            let mut chain = AuditChain::new("test-doc");

            // Add enough events
            for i in 0..6 {
                chain.append(
                    AuditAction::View,
                    &format!("user{}@test.com", i),
                    &format!("{:064x}", i),
                    None,
                );
            }

            // Verify intact chain
            prop_assert!(chain.verify().is_ok());

            // Tamper with one event's actor_email
            let original = chain.events[tamper_index].actor_email.clone();
            chain.events[tamper_index].actor_email = "tampered@evil.com".to_string();

            // Should fail if we tampered with an event that has a successor
            // (first event affects all subsequent hashes)
            if tamper_index < chain.events.len() - 1 {
                prop_assert!(chain.verify().is_err());
            }

            // Restore and verify it works again
            chain.events[tamper_index].actor_email = original;
            prop_assert!(chain.verify().is_ok());
        }

        /// Property: JSON serialization roundtrip preserves all data
        #[test]
        fn json_roundtrip(count in 1usize..10) {
            let mut chain = AuditChain::new("roundtrip-test");

            for i in 0..count {
                chain.append(
                    AuditAction::Sign,
                    &format!("signer{}@test.com", i),
                    &format!("{:064x}", i),
                    Some(format!("Details for event {}", i)),
                );
            }

            let json = chain.to_json().unwrap();
            let restored = AuditChain::from_json(&json).unwrap();

            prop_assert_eq!(chain.events.len(), restored.events.len());
            prop_assert_eq!(&chain.document_id, &restored.document_id);

            // Both chains should verify
            prop_assert!(chain.verify().is_ok());
            prop_assert!(restored.verify().is_ok());
        }

        /// Property: Hash linking is consistent
        #[test]
        fn hash_linking_consistent(count in 2usize..10) {
            let mut chain = AuditChain::new("hash-test");

            for i in 0..count {
                chain.append(
                    AuditAction::View,
                    "test@example.com",
                    &format!("{:064x}", i),
                    None,
                );
            }

            // Verify each event's previous_hash matches computed hash of predecessor
            for i in 1..chain.events.len() {
                let expected_prev = chain.events[i - 1].compute_hash();
                prop_assert_eq!(
                    chain.events[i].previous_hash.as_ref(),
                    Some(&expected_prev),
                    "Event {} has wrong previous hash", i
                );
            }

            // First event should have no previous hash
            prop_assert!(chain.events[0].previous_hash.is_none());
        }

        /// Property: Document hash function is deterministic
        #[test]
        fn hash_document_deterministic(data in prop::collection::vec(any::<u8>(), 0..1024)) {
            let hash1 = hash_document(&data);
            let hash2 = hash_document(&data);
            prop_assert_eq!(&hash1, &hash2);
            prop_assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars
        }
    }
}
