//! State-specific compliance rules
//!
//! Each state module implements compliance checks for that jurisdiction's
//! landlord-tenant statutes.

pub mod florida;
pub mod texas;

use crate::jurisdiction::State;
use shared_types::Violation;

/// Get state-specific compliance violations
pub fn check_state_compliance(state: State, text: &str) -> Vec<Violation> {
    match state {
        State::FL => florida::check_florida_compliance(text),
        State::TX => texas::check_texas_compliance(text),
        // States without implementation return empty
        _ => Vec::new(),
    }
}

/// Check if state has full compliance implementation
pub fn has_implementation(state: State) -> bool {
    matches!(state, State::FL | State::TX)
}

/// Get list of statutes/codes covered for a state
pub fn covered_statutes(state: State) -> Vec<&'static str> {
    match state {
        State::FL => vec![
            "F.S. § 83.47 - Prohibited provisions",
            "F.S. § 83.48 - Attorney fees reciprocity",
            "F.S. § 83.49 - Security deposits",
            "F.S. § 83.51 - Landlord obligations",
            "F.S. § 83.56 - Termination notices",
            "F.S. § 83.57 - Month-to-month tenancy",
        ],
        State::TX => vec![
            "Tex. Prop. Code § 92.001-92.355 - Landlord-tenant",
            "Tex. Prop. Code § 92.104 - Security deposit return",
            "Tex. Prop. Code § 92.0081 - Lockout requirements",
            "Tex. Prop. Code § 92.3515 - Screening criteria notice",
            "Tex. Prop. Code § 92.056 - Landlord repair duties",
        ],
        _ => vec![],
    }
}
