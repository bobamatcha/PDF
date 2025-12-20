//! Audit Trail Display for WASM
//!
//! Provides display-friendly representations of AuditChain data
//! for the frontend timeline view.

use serde::{Deserialize, Serialize};
use shared_types::audit::{AuditAction, AuditChain};
use wasm_bindgen::prelude::*;

/// Timeline event for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub timestamp: String,
    pub action_type: String,
    pub description: String,
    pub actor: String,
    pub details: Option<String>,
}

/// Summary statistics for the audit chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub chain_valid: bool,
    pub first_event_time: Option<String>,
    pub last_event_time: Option<String>,
    pub event_types: Vec<String>,
}

/// Display wrapper for AuditChain
#[wasm_bindgen]
pub struct AuditDisplay {
    chain: Option<AuditChain>,
}

impl AuditDisplay {
    /// Create a new AuditDisplay
    pub fn new() -> Self {
        Self { chain: None }
    }

    /// Set the audit chain to display
    pub fn set_chain(&mut self, chain: AuditChain) {
        self.chain = Some(chain);
    }

    /// Get the number of events in the chain
    pub fn event_count(&self) -> usize {
        self.chain.as_ref().map(|c| c.events.len()).unwrap_or(0)
    }

    /// Check if the chain is valid (integrity check)
    pub fn is_chain_valid(&self) -> bool {
        self.chain
            .as_ref()
            .map(|c| c.verify().is_ok())
            .unwrap_or(false)
    }

    /// Convert the chain to timeline JSON
    pub fn to_timeline_json(&self) -> String {
        if let Some(chain) = &self.chain {
            let events: Vec<TimelineEvent> = chain
                .events
                .iter()
                .map(|e| TimelineEvent {
                    id: e.event_id.clone(),
                    timestamp: e.timestamp.clone(),
                    action_type: format!("{:?}", e.action),
                    description: Self::describe_action(&e.action),
                    actor: e.actor_email.clone(),
                    details: e.details.clone(),
                })
                .collect();

            serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> AuditSummary {
        if let Some(chain) = &self.chain {
            let mut event_types = Vec::new();
            let mut seen_types = std::collections::HashSet::new();

            for event in &chain.events {
                let type_name = format!("{:?}", event.action);
                if seen_types.insert(type_name.clone()) {
                    event_types.push(type_name);
                }
            }

            AuditSummary {
                total_events: chain.events.len(),
                chain_valid: chain.verify().is_ok(),
                first_event_time: chain.events.first().map(|e| e.timestamp.clone()),
                last_event_time: chain.events.last().map(|e| e.timestamp.clone()),
                event_types,
            }
        } else {
            AuditSummary {
                total_events: 0,
                chain_valid: true,
                first_event_time: None,
                last_event_time: None,
                event_types: Vec::new(),
            }
        }
    }

    /// Convert action to human-readable description
    fn describe_action(action: &AuditAction) -> String {
        match action {
            AuditAction::Upload => "Document uploaded".to_string(),
            AuditAction::View => "Document viewed".to_string(),
            AuditAction::FieldAdded => "Field added".to_string(),
            AuditAction::FieldRemoved => "Field removed".to_string(),
            AuditAction::Sign => "Document signed".to_string(),
            AuditAction::Decline => "Document declined".to_string(),
            AuditAction::Complete => "Document completed".to_string(),
            AuditAction::Send => "Document sent".to_string(),
            AuditAction::ComplianceCheck { violations_found } => {
                format!("Compliance check: {} violation(s) found", violations_found)
            }
            AuditAction::DocumentLoaded { hash } => {
                let short_hash = if hash.len() >= 8 { &hash[..8] } else { hash };
                format!("Document loaded (hash: {})", short_hash)
            }
            AuditAction::FieldAddedDetailed { field_type, page } => {
                format!("Added {} field on page {}", field_type, page)
            }
            AuditAction::FieldMoved {
                field_id,
                new_x,
                new_y,
            } => {
                format!("Moved field {} to ({:.1}, {:.1})", field_id, new_x, new_y)
            }
            AuditAction::FieldDeleted { field_id } => {
                format!("Deleted field {}", field_id)
            }
        }
    }
}

impl Default for AuditDisplay {
    fn default() -> Self {
        Self::new()
    }
}

// WASM bindings
#[wasm_bindgen]
impl AuditDisplay {
    /// Create a new AuditDisplay instance (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> Self {
        Self::new()
    }

    /// Get timeline events as JSON string (WASM)
    #[wasm_bindgen(js_name = getTimelineJson)]
    pub fn get_timeline_json_wasm(&self) -> String {
        self.to_timeline_json()
    }

    /// Get summary statistics as JSON string (WASM)
    #[wasm_bindgen(js_name = getSummaryJson)]
    pub fn get_summary_json(&self) -> String {
        let summary = self.get_summary();
        serde_json::to_string(&summary).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if the chain is valid (WASM)
    #[wasm_bindgen(js_name = isValid)]
    pub fn is_valid_wasm(&self) -> bool {
        self.is_chain_valid()
    }

    /// Load chain from JSON (WASM)
    #[wasm_bindgen(js_name = loadChainJson)]
    pub fn load_chain_json(&mut self, json: &str) -> Result<(), JsValue> {
        let chain = AuditChain::from_json(json).map_err(|e| JsValue::from_str(&e))?;
        self.chain = Some(chain);
        Ok(())
    }

    /// Record a new action (WASM)
    #[wasm_bindgen(js_name = recordAction)]
    pub fn record_action_wasm(&mut self, action_json: &str) -> Result<(), JsValue> {
        #[derive(Deserialize)]
        struct ActionRecord {
            action_type: String,
            actor_email: String,
            document_hash: String,
            details: Option<String>,
            // Additional fields for specific actions
            violations_found: Option<u32>,
            field_type: Option<String>,
            page: Option<u32>,
            field_id: Option<String>,
            new_x: Option<f64>,
            new_y: Option<f64>,
        }

        let record: ActionRecord = serde_json::from_str(action_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse action: {}", e)))?;

        // If no chain exists, create one
        if self.chain.is_none() {
            self.chain = Some(AuditChain::new("default-doc"));
        }

        let action = match record.action_type.as_str() {
            "Upload" => AuditAction::Upload,
            "View" => AuditAction::View,
            "FieldAdded" => AuditAction::FieldAdded,
            "FieldRemoved" => AuditAction::FieldRemoved,
            "Sign" => AuditAction::Sign,
            "Decline" => AuditAction::Decline,
            "Complete" => AuditAction::Complete,
            "Send" => AuditAction::Send,
            "ComplianceCheck" => AuditAction::ComplianceCheck {
                violations_found: record.violations_found.unwrap_or(0),
            },
            "DocumentLoaded" => AuditAction::DocumentLoaded {
                hash: record.document_hash.clone(),
            },
            "FieldAddedDetailed" => AuditAction::FieldAddedDetailed {
                field_type: record.field_type.unwrap_or_default(),
                page: record.page.unwrap_or(1),
            },
            "FieldMoved" => AuditAction::FieldMoved {
                field_id: record.field_id.unwrap_or_default(),
                new_x: record.new_x.unwrap_or(0.0),
                new_y: record.new_y.unwrap_or(0.0),
            },
            "FieldDeleted" => AuditAction::FieldDeleted {
                field_id: record.field_id.unwrap_or_default(),
            },
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Unknown action type: {}",
                    record.action_type
                )))
            }
        };

        if let Some(chain) = &mut self.chain {
            chain.append(
                action,
                &record.actor_email,
                &record.document_hash,
                record.details,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::audit::{AuditAction, AuditChain};

    #[test]
    fn test_audit_display_creation() {
        let display = AuditDisplay::new();
        assert_eq!(display.event_count(), 0);
    }

    #[test]
    fn test_set_audit_chain() {
        let mut display = AuditDisplay::new();
        let mut chain = AuditChain::new("test-doc");
        chain.append(
            AuditAction::DocumentLoaded {
                hash: "abc123".to_string(),
            },
            "user@example.com",
            "abc123",
            None,
        );
        display.set_chain(chain);
        assert_eq!(display.event_count(), 1);
    }

    #[test]
    fn test_audit_chain_records_compliance_check() {
        let mut chain = AuditChain::new("test-doc");
        chain.append(
            AuditAction::ComplianceCheck {
                violations_found: 3,
            },
            "compliance@example.com",
            "hash1",
            None,
        );
        assert!(chain.verify().is_ok());
        assert_eq!(chain.events.len(), 1);
    }

    #[test]
    fn test_to_timeline_json() {
        let mut display = AuditDisplay::new();
        let mut chain = AuditChain::new("test-doc");
        chain.append(
            AuditAction::DocumentLoaded {
                hash: "abc123".to_string(),
            },
            "user@example.com",
            "abc123",
            None,
        );
        chain.append(
            AuditAction::ComplianceCheck {
                violations_found: 2,
            },
            "compliance@example.com",
            "abc123",
            None,
        );
        display.set_chain(chain);

        let json = display.to_timeline_json();
        assert!(json.contains("DocumentLoaded"));
        assert!(json.contains("ComplianceCheck"));
    }

    #[test]
    fn test_chain_integrity_display() {
        let mut display = AuditDisplay::new();
        let mut chain = AuditChain::new("test-doc");
        chain.append(
            AuditAction::DocumentLoaded {
                hash: "abc123".to_string(),
            },
            "user@example.com",
            "abc123",
            None,
        );
        display.set_chain(chain);

        assert!(display.is_chain_valid());
    }

    #[test]
    fn test_get_events_summary() {
        let mut display = AuditDisplay::new();
        let mut chain = AuditChain::new("test-doc");
        chain.append(
            AuditAction::DocumentLoaded {
                hash: "abc123".to_string(),
            },
            "user@example.com",
            "abc123",
            None,
        );
        chain.append(
            AuditAction::ComplianceCheck {
                violations_found: 5,
            },
            "compliance@example.com",
            "abc123",
            None,
        );
        chain.append(
            AuditAction::FieldAddedDetailed {
                field_type: "signature".to_string(),
                page: 1,
            },
            "user@example.com",
            "abc123",
            None,
        );
        display.set_chain(chain);

        let summary = display.get_summary();
        assert!(summary.total_events >= 3);
        assert!(summary.chain_valid);
        assert!(summary.first_event_time.is_some());
        assert!(summary.last_event_time.is_some());
    }
}
