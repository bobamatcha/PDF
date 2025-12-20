//! Compliance rules for lease verification
//!
//! This module contains state-specific and federal compliance rules
//! for verifying lease documents.

pub mod florida;

use crate::verifier::parser::ParsedLease;
use serde::{Deserialize, Serialize};

/// Trait for compliance rules
pub trait ComplianceRule: Send + Sync {
    /// Human-readable name of the rule
    fn name(&self) -> &str;

    /// Statute or regulation reference (e.g., "F.S. § 83.49")
    fn statute_reference(&self) -> &str;

    /// Check the lease against this rule
    fn check(&self, lease: &ParsedLease) -> RuleResult;
}

/// Result of a compliance rule check
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleResult {
    /// Rule passed
    Pass,

    /// Rule failed
    Fail {
        /// Explanation of why it failed
        reason: String,
        /// Severity of the failure
        severity: Severity,
    },

    /// Warning - not a failure but should be reviewed
    Warning {
        /// Explanation of the warning
        reason: String,
    },

    /// Rule is not applicable to this document
    NotApplicable,
}

impl RuleResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, RuleResult::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, RuleResult::Fail { .. })
    }

    pub fn is_warning(&self) -> bool {
        matches!(self, RuleResult::Warning { .. })
    }
}

/// Severity of a rule failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Lease may be unenforceable
    Critical,
    /// Statutory violation
    High,
    /// Best practice violation
    Medium,
    /// Minor issue
    Low,
}

/// Get all Florida-specific compliance rules
pub fn get_florida_rules() -> Vec<Box<dyn ComplianceRule>> {
    florida::get_all_rules()
}
