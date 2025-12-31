//! State-specific compliance rules
//!
//! Each state module implements compliance checks for that jurisdiction's
//! landlord-tenant statutes and real estate transaction requirements.
//!
//! ## Feature Flags
//!
//! - `florida-only` (default): Only Florida compliance rules are active
//! - `all-states`: All 16 state implementations are active

// Florida - Always active (MVP focus)
pub mod florida;
pub mod florida_billofsale;
pub mod florida_contractor;
pub mod florida_realestate;

// Non-Florida states - Only compiled with `all-states` feature
#[cfg(feature = "all-states")]
pub mod arizona;
#[cfg(feature = "all-states")]
pub mod california;
#[cfg(feature = "all-states")]
pub mod georgia;
#[cfg(feature = "all-states")]
pub mod illinois;
#[cfg(feature = "all-states")]
pub mod massachusetts;
#[cfg(feature = "all-states")]
pub mod michigan;
#[cfg(feature = "all-states")]
pub mod new_jersey;
#[cfg(feature = "all-states")]
pub mod new_york;
#[cfg(feature = "all-states")]
pub mod north_carolina;
#[cfg(feature = "all-states")]
pub mod ohio;
#[cfg(feature = "all-states")]
pub mod pennsylvania;
#[cfg(feature = "all-states")]
pub mod tennessee;
#[cfg(feature = "all-states")]
pub mod texas;
#[cfg(feature = "all-states")]
pub mod virginia;
#[cfg(feature = "all-states")]
pub mod washington;

use crate::jurisdiction::State;
use shared_types::Violation;

/// Get state-specific compliance violations
///
/// Layer 2 of the Layer Cake: State statutory requirements.
/// Local ordinances (Chicago RLTO, NYC rent control) are handled in Layer 3.
pub fn check_state_compliance(state: State, text: &str) -> Vec<Violation> {
    match state {
        // Florida - Always active
        State::FL => florida::check_florida_compliance(text),

        // Non-Florida states - Only with all-states feature
        #[cfg(feature = "all-states")]
        State::TX => texas::check_texas_compliance(text),
        #[cfg(feature = "all-states")]
        State::CA => california::check_california_compliance(text),
        #[cfg(feature = "all-states")]
        State::NY => new_york::check_new_york_compliance(text),
        #[cfg(feature = "all-states")]
        State::GA => georgia::check_georgia_compliance(text),
        #[cfg(feature = "all-states")]
        State::IL => illinois::check_illinois_compliance(text),
        #[cfg(feature = "all-states")]
        State::PA => pennsylvania::check_pennsylvania_compliance(text),
        #[cfg(feature = "all-states")]
        State::NJ => new_jersey::check_new_jersey_compliance(text),
        #[cfg(feature = "all-states")]
        State::VA => virginia::check_virginia_compliance(text),
        #[cfg(feature = "all-states")]
        State::MA => massachusetts::check_massachusetts_compliance(text),
        #[cfg(feature = "all-states")]
        State::OH => ohio::check_ohio_compliance(text),
        #[cfg(feature = "all-states")]
        State::MI => michigan::check_michigan_compliance(text),
        #[cfg(feature = "all-states")]
        State::WA => washington::check_washington_compliance(text),
        #[cfg(feature = "all-states")]
        State::AZ => arizona::check_arizona_compliance(text),
        #[cfg(feature = "all-states")]
        State::NC => north_carolina::check_north_carolina_compliance(text),
        #[cfg(feature = "all-states")]
        State::TN => tennessee::check_tennessee_compliance(text),

        // States without implementation (or feature disabled) return empty
        _ => Vec::new(),
    }
}

/// Check if state has full compliance implementation
pub fn has_implementation(state: State) -> bool {
    #[cfg(feature = "all-states")]
    {
        matches!(
            state,
            State::FL
                | State::TX
                | State::CA
                | State::NY
                | State::GA
                | State::IL
                | State::PA
                | State::NJ
                | State::VA
                | State::MA
                | State::OH
                | State::MI
                | State::WA
                | State::AZ
                | State::NC
                | State::TN
        )
    }

    #[cfg(not(feature = "all-states"))]
    {
        matches!(state, State::FL)
    }
}

/// Get list of statutes/codes covered for a state
pub fn covered_statutes(state: State) -> Vec<&'static str> {
    match state {
        State::FL => vec![
            // Lease/Landlord-Tenant
            "F.S. § 83.47 - Prohibited provisions",
            "F.S. § 83.48 - Attorney fees reciprocity",
            "F.S. § 83.49 - Security deposits",
            "F.S. § 83.51 - Landlord obligations",
            "F.S. § 83.56 - Termination notices",
            "F.S. § 83.57 - Month-to-month tenancy",
            // Real Estate Transactions
            "F.S. § 404.056 - Radon Gas Disclosure",
            "F.S. § 689.261 - Property Tax Disclosure",
            "F.S. § 689.302 - Flood Disclosure (SB 948)",
            "F.S. § 720.401 - HOA Disclosure",
            "F.S. § 553.996 - Energy Efficiency Disclosure",
            "F.S. § 475.278 - Brokerage Relationship Disclosure",
            "F.S. § 475.25 - Listing Agreement Expiration",
            // Construction Liens (Phase 1.0.4)
            "F.S. § 713.13 - Notice of Commencement",
            "F.S. § 713.06 - Notice to Owner",
            "F.S. § 713.31 - Fraudulent Liens",
            // Motor Vehicles (Phase 1.1)
            "F.S. § 319 - Motor Vehicle Titles",
        ],
        #[cfg(feature = "all-states")]
        State::TX => vec![
            "Tex. Prop. Code § 92.001-92.355 - Landlord-tenant",
            "Tex. Prop. Code § 92.104 - Security deposit return",
            "Tex. Prop. Code § 92.0081 - Lockout requirements",
            "Tex. Prop. Code § 92.3515 - Screening criteria notice",
            "Tex. Prop. Code § 92.056 - Landlord repair duties",
        ],
        #[cfg(feature = "all-states")]
        State::CA => vec![
            "CA Civil Code § 1950.5 - Security deposits (AB 12)",
            "CA Civil Code § 1953 - Void lease provisions",
            "CA Civil Code § 1946.2 - Just Cause (AB 1482)",
            "CA Civil Code § 1946.2 - Junk fees (SB 611)",
            "CA Civil Code § 827 - Rent increase notice",
        ],
        #[cfg(feature = "all-states")]
        State::NY => vec![
            "NY RPL § 238-a - Late fee cap ($50 or 5%)",
            "NY RPL § 7-108 - Security deposit (1 month max)",
            "NY RPL § 226-c - Good Cause eviction",
            "NYC Admin Code § 26-504 - Rent stabilization",
            "NY GOL § 5-321 - Void liability waivers",
        ],
        #[cfg(feature = "all-states")]
        State::GA => vec![
            "GA Code § 44-7-13 - Habitability (HB 404)",
            "GA Code § 44-7-50 - Notice requirements",
            "GA Code § 44-7-30 - Security deposits",
            "GA Code § 44-7-20 - Flooding disclosure",
            "GA Code § 44-7-33 - Move-in inspection",
        ],
        #[cfg(feature = "all-states")]
        State::IL => vec![
            "765 ILCS 705 - Landlord obligations",
            "765 ILCS 710 - Security deposits",
            "765 ILCS 720 - Retaliation protection",
            "Chicago Mun. Code § 5-12-170 - RLTO Summary",
            "Chicago Mun. Code § 5-12-080 - Deposit interest",
        ],
        #[cfg(feature = "all-states")]
        State::PA => vec![
            "68 P.S. § 250.511a - Security deposit limits",
            "68 P.S. § 250.512 - Deposit return (30 days)",
            "73 P.S. § 2205 - Plain Language Act",
            "68 P.S. § 250.511b - Deposit interest",
        ],
        #[cfg(feature = "all-states")]
        State::NJ => vec![
            "N.J.S.A. 46:8-45 - Truth in Renting Statement",
            "N.J.S.A. 46:8-21.2 - Security deposit (1.5 months)",
            "N.J.S.A. 46:8-19 - Interest-bearing account",
            "N.J.S.A. 2A:18-61.1 - Anti-Eviction Act",
        ],
        #[cfg(feature = "all-states")]
        State::VA => vec![
            "VA Code § 55.1-1204 - Fee transparency (HB 2430)",
            "VA Code § 55.1-1226 - Security deposit (2 months)",
            "VA Code § 55.1-1215 - Mold disclosure",
            "VA Code § 55.1-1214 - Move-in inspection",
        ],
        #[cfg(feature = "all-states")]
        State::MA => vec![
            "M.G.L. c. 186 § 15B - Security deposit (1 month)",
            "M.G.L. c. 186 (2025) - Broker fee reform",
            "M.G.L. c. 186 § 14 - Quiet enjoyment",
        ],
        #[cfg(feature = "all-states")]
        State::OH => vec![
            "O.R.C. § 5321.16 - Security deposit return (30 days)",
            "O.R.C. § 5321.04 - Landlord obligations",
            "O.R.C. § 5321.06 - Prohibited provisions",
        ],
        #[cfg(feature = "all-states")]
        State::MI => vec![
            "M.C.L. 37.2502a - Source of income protection",
            "M.C.L. 554.602 - Security deposit (1.5 months)",
            "M.C.L. 554.608 - Inventory checklist",
            "M.C.L. 554.633 - Void provisions",
        ],
        #[cfg(feature = "all-states")]
        State::WA => vec![
            "RCW 59.18.140 - 90-day rent increase notice",
            "RCW 59.18.260 - Security deposit requirements",
            "RCW 59.18.280 - 21-day deposit return",
            "Seattle Mun. Code 22.206 - Just Cause eviction",
        ],
        #[cfg(feature = "all-states")]
        State::AZ => vec![
            "A.R.S. § 33-1319 - Bed bug disclosure",
            "A.R.S. § 33-1321 - Security deposit (1.5 months)",
            "A.R.S. § 36-1681 - Pool safety notice",
            "A.R.S. § 33-1315 - Prohibited provisions",
        ],
        #[cfg(feature = "all-states")]
        State::NC => vec![
            "N.C.G.S. § 42-51 - Security deposit (2 months)",
            "N.C.G.S. § 42-53 - Pet deposit/fee distinction",
            "N.C.G.S. § 42-50 - Trust account requirement",
            "N.C.G.S. § 42-46 - Prohibited provisions",
        ],
        #[cfg(feature = "all-states")]
        State::TN => vec![
            "T.C.A. § 66-28-102 - URLTA applicability",
            "T.C.A. § 66-28-301 - Security deposit requirements",
            "T.C.A. § 66-28-505 - 14-day nonpayment notice",
            "T.C.A. § 66-28-104 - Prohibited provisions",
        ],
        _ => vec![],
    }
}
