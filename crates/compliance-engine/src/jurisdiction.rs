//! Jurisdiction types for multi-state compliance checking
//!
//! Implements the "Layer Cake" architecture:
//! - Federal (baseline): Lead paint, Fair Housing
//! - State: Statutory requirements per state
//! - Local: Municipal ordinances (Chicago RLTO, NYC rent control, etc.)

use serde::{Deserialize, Serialize};

/// US State codes for lease compliance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum State {
    // Tier 0: Current
    FL,
    // Tier 1: Big Five (priority order from research)
    TX,
    CA,
    NY,
    GA,
    IL,
    // Tier 2: Growth Hubs
    PA,
    NJ,
    VA,
    MA,
    OH,
    MI,
    WA,
    AZ,
    NC,
    TN,
    // Tier 3: URLTA Block
    AK,
    KS,
    KY,
    NE,
    NM,
    OR,
    RI,
    // Other states (alphabetical)
    AL,
    AR,
    CO,
    CT,
    DE,
    HI,
    IA,
    ID,
    IN,
    LA,
    MD,
    ME,
    MN,
    MO,
    MS,
    MT,
    ND,
    NH,
    NV,
    OK,
    SC,
    SD,
    UT,
    VT,
    WI,
    WV,
    WY,
    DC,
}

impl State {
    /// Get the full state name
    pub fn name(&self) -> &'static str {
        match self {
            State::FL => "Florida",
            State::TX => "Texas",
            State::CA => "California",
            State::NY => "New York",
            State::GA => "Georgia",
            State::IL => "Illinois",
            State::PA => "Pennsylvania",
            State::NJ => "New Jersey",
            State::VA => "Virginia",
            State::MA => "Massachusetts",
            State::OH => "Ohio",
            State::MI => "Michigan",
            State::WA => "Washington",
            State::AZ => "Arizona",
            State::NC => "North Carolina",
            State::TN => "Tennessee",
            State::AK => "Alaska",
            State::KS => "Kansas",
            State::KY => "Kentucky",
            State::NE => "Nebraska",
            State::NM => "New Mexico",
            State::OR => "Oregon",
            State::RI => "Rhode Island",
            State::AL => "Alabama",
            State::AR => "Arkansas",
            State::CO => "Colorado",
            State::CT => "Connecticut",
            State::DE => "Delaware",
            State::HI => "Hawaii",
            State::IA => "Iowa",
            State::ID => "Idaho",
            State::IN => "Indiana",
            State::LA => "Louisiana",
            State::MD => "Maryland",
            State::ME => "Maine",
            State::MN => "Minnesota",
            State::MO => "Missouri",
            State::MS => "Mississippi",
            State::MT => "Montana",
            State::ND => "North Dakota",
            State::NH => "New Hampshire",
            State::NV => "Nevada",
            State::OK => "Oklahoma",
            State::SC => "South Carolina",
            State::SD => "South Dakota",
            State::UT => "Utah",
            State::VT => "Vermont",
            State::WI => "Wisconsin",
            State::WV => "West Virginia",
            State::WY => "Wyoming",
            State::DC => "District of Columbia",
        }
    }

    /// Get the primary statute citation for landlord-tenant law
    /// Returns the main statutory reference for implemented states
    pub fn statute_citation(&self) -> Option<&'static str> {
        match self {
            // Tier 0 + Tier 1: Big Five
            State::FL => Some("F.S. Chapter 83"),
            State::TX => Some("Tex. Prop. Code Ch. 92"),
            State::CA => Some("CA Civil Code 1940-1954"),
            State::NY => Some("NY RPL Article 7"),
            State::GA => Some("GA Code Title 44 Ch. 7"),
            State::IL => Some("765 ILCS + Chicago RLTO"),
            // Tier 2: Growth Hubs
            State::PA => Some("68 P.S. § 250.501 et seq."),
            State::NJ => Some("N.J.S.A. 46:8 et seq."),
            State::VA => Some("VA Code § 55.1-1200 et seq."),
            State::MA => Some("M.G.L. c. 186"),
            State::OH => Some("O.R.C. Chapter 5321"),
            State::MI => Some("M.C.L. 554.601 et seq."),
            State::WA => Some("RCW 59.18"),
            State::AZ => Some("A.R.S. Title 33 Ch. 10"),
            State::NC => Some("N.C.G.S. Chapter 42"),
            State::TN => Some("T.C.A. Title 66 Ch. 28"),
            // Not yet implemented
            _ => None,
        }
    }

    /// Check if state follows URLTA (Uniform Residential Landlord and Tenant Act)
    pub fn is_urlta_state(&self) -> bool {
        matches!(
            self,
            State::AK
                | State::AZ
                | State::CT
                | State::FL
                | State::HI
                | State::IA
                | State::KS
                | State::KY
                | State::MT
                | State::NE
                | State::NM
                | State::OK
                | State::OR
                | State::RI
                | State::SC
                | State::TN
                | State::VA
                | State::WA
        )
    }

    /// Get the tier for rollout prioritization
    pub fn tier(&self) -> Tier {
        match self {
            State::FL => Tier::Zero, // Already implemented
            State::TX | State::CA | State::NY | State::GA | State::IL => Tier::One,
            State::PA
            | State::NJ
            | State::VA
            | State::MA
            | State::OH
            | State::MI
            | State::WA
            | State::AZ
            | State::NC
            | State::TN => Tier::Two,
            State::AK | State::KS | State::KY | State::NE | State::NM | State::OR | State::RI => {
                Tier::Three
            }
            _ => Tier::Four,
        }
    }

    /// Check if state has implementation
    pub fn is_implemented(&self) -> bool {
        matches!(
            self,
            // Tier 1: Big Five
            State::FL
                | State::TX
                | State::CA
                | State::NY
                | State::GA
                | State::IL
                // Tier 2: Growth Hubs
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

    /// Parse from state code or name (case-insensitive)
    pub fn parse_code(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "FL" | "FLORIDA" => Some(State::FL),
            "TX" | "TEXAS" => Some(State::TX),
            "CA" | "CALIFORNIA" => Some(State::CA),
            "NY" | "NEW YORK" => Some(State::NY),
            "GA" | "GEORGIA" => Some(State::GA),
            "IL" | "ILLINOIS" => Some(State::IL),
            "PA" | "PENNSYLVANIA" => Some(State::PA),
            "NJ" | "NEW JERSEY" => Some(State::NJ),
            "VA" | "VIRGINIA" => Some(State::VA),
            "MA" | "MASSACHUSETTS" => Some(State::MA),
            "OH" | "OHIO" => Some(State::OH),
            "MI" | "MICHIGAN" => Some(State::MI),
            "WA" | "WASHINGTON" => Some(State::WA),
            "AZ" | "ARIZONA" => Some(State::AZ),
            "NC" | "NORTH CAROLINA" => Some(State::NC),
            "TN" | "TENNESSEE" => Some(State::TN),
            "AK" | "ALASKA" => Some(State::AK),
            "KS" | "KANSAS" => Some(State::KS),
            "KY" | "KENTUCKY" => Some(State::KY),
            "NE" | "NEBRASKA" => Some(State::NE),
            "NM" | "NEW MEXICO" => Some(State::NM),
            "OR" | "OREGON" => Some(State::OR),
            "RI" | "RHODE ISLAND" => Some(State::RI),
            "DC" | "DISTRICT OF COLUMBIA" => Some(State::DC),
            _ => None,
        }
    }

    /// Get all implemented states
    pub fn implemented_states() -> Vec<Self> {
        vec![
            // Tier 1: Big Five
            State::FL,
            State::TX,
            State::CA,
            State::NY,
            State::GA,
            State::IL,
            // Tier 2: Growth Hubs
            State::PA,
            State::NJ,
            State::VA,
            State::MA,
            State::OH,
            State::MI,
            State::WA,
            State::AZ,
            State::NC,
            State::TN,
        ]
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Rollout tier based on volume/complexity matrix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Tier 0: Already implemented (FL)
    Zero,
    /// Tier 1: Big Five - essential anchors
    One,
    /// Tier 2: Growth Hubs - regional importance
    Two,
    /// Tier 3: URLTA Block - clone master template
    Three,
    /// Tier 4: Long Tail - remaining states
    Four,
}

/// Full jurisdiction including state and optional locality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Jurisdiction {
    pub state: State,
    pub locality: Option<Locality>,
}

impl Jurisdiction {
    pub fn new(state: State) -> Self {
        Self {
            state,
            locality: None,
        }
    }

    pub fn with_locality(state: State, locality: Locality) -> Self {
        Self {
            state,
            locality: Some(locality),
        }
    }

    /// Create jurisdiction from zip code (for local ordinance detection)
    pub fn from_zip(state: State, zip: &str) -> Self {
        let locality = Locality::from_zip(state, zip);
        Self { state, locality }
    }

    /// Get the jurisdiction ID string (e.g., "US-IL-CHICAGO")
    pub fn id(&self) -> String {
        match &self.locality {
            Some(loc) => format!("US-{}-{}", self.state, loc.code()),
            None => format!("US-{}", self.state),
        }
    }
}

/// Known localities with special ordinances
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Locality {
    // Illinois
    Chicago,
    // California
    SanFrancisco,
    LosAngeles,
    SantaMonica,
    WestHollywood,
    Oakland,
    Berkeley,
    // New York
    NewYorkCity,
    // Other cities with rent control or special rules
    WashingtonDC,
    Custom(String),
}

impl Locality {
    pub fn code(&self) -> &str {
        match self {
            Locality::Chicago => "CHICAGO",
            Locality::SanFrancisco => "SF",
            Locality::LosAngeles => "LA",
            Locality::SantaMonica => "SANTA_MONICA",
            Locality::WestHollywood => "WEHO",
            Locality::Oakland => "OAKLAND",
            Locality::Berkeley => "BERKELEY",
            Locality::NewYorkCity => "NYC",
            Locality::WashingtonDC => "DC",
            Locality::Custom(s) => s,
        }
    }

    /// Detect locality from zip code
    pub fn from_zip(state: State, zip: &str) -> Option<Self> {
        match state {
            State::IL => {
                // Chicago zip codes: 60601-60661, 60701-60707
                if let Ok(z) = zip.parse::<u32>() {
                    if (60601..=60661).contains(&z) || (60701..=60707).contains(&z) {
                        return Some(Locality::Chicago);
                    }
                }
                None
            }
            State::NY => {
                // NYC zip codes: 10001-10292, 10301-10314 (Staten Island), 11xxx (Brooklyn/Queens)
                if let Ok(z) = zip.parse::<u32>() {
                    if (10001..=10292).contains(&z)
                        || (10301..=10314).contains(&z)
                        || (11001..=11697).contains(&z)
                    {
                        return Some(Locality::NewYorkCity);
                    }
                }
                None
            }
            State::CA => {
                // San Francisco: 94102-94188
                // Los Angeles: 90001-90189, 90201-90899
                // Santa Monica: 90401-90411
                if let Ok(z) = zip.parse::<u32>() {
                    if (94102..=94188).contains(&z) {
                        return Some(Locality::SanFrancisco);
                    }
                    if (90401..=90411).contains(&z) {
                        return Some(Locality::SantaMonica);
                    }
                    if (90001..=90189).contains(&z) || (90201..=90899).contains(&z) {
                        return Some(Locality::LosAngeles);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if locality requires RLTO Summary (Chicago)
    pub fn requires_rlto(&self) -> bool {
        matches!(self, Locality::Chicago)
    }

    /// Check if locality has rent control
    pub fn has_rent_control(&self) -> bool {
        matches!(
            self,
            Locality::SanFrancisco
                | Locality::LosAngeles
                | Locality::SantaMonica
                | Locality::WestHollywood
                | Locality::Oakland
                | Locality::Berkeley
                | Locality::NewYorkCity
                | Locality::WashingtonDC
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_parsing() {
        assert_eq!(State::parse_code("FL"), Some(State::FL));
        assert_eq!(State::parse_code("florida"), Some(State::FL));
        assert_eq!(State::parse_code("TX"), Some(State::TX));
        assert_eq!(State::parse_code("texas"), Some(State::TX));
    }

    #[test]
    fn test_state_tiers() {
        assert_eq!(State::FL.tier(), Tier::Zero);
        assert_eq!(State::TX.tier(), Tier::One);
        assert_eq!(State::CA.tier(), Tier::One);
        assert_eq!(State::PA.tier(), Tier::Two);
        assert_eq!(State::AK.tier(), Tier::Three);
    }

    #[test]
    fn test_urlta_states() {
        assert!(State::FL.is_urlta_state());
        assert!(State::AK.is_urlta_state());
        assert!(!State::NY.is_urlta_state());
    }

    #[test]
    fn test_locality_from_zip() {
        assert_eq!(
            Locality::from_zip(State::IL, "60601"),
            Some(Locality::Chicago)
        );
        assert_eq!(
            Locality::from_zip(State::NY, "10001"),
            Some(Locality::NewYorkCity)
        );
        assert_eq!(
            Locality::from_zip(State::CA, "94102"),
            Some(Locality::SanFrancisco)
        );
        assert_eq!(Locality::from_zip(State::TX, "75001"), None);
    }

    #[test]
    fn test_jurisdiction_id() {
        let fl = Jurisdiction::new(State::FL);
        assert_eq!(fl.id(), "US-FL");

        let chicago = Jurisdiction::with_locality(State::IL, Locality::Chicago);
        assert_eq!(chicago.id(), "US-IL-CHICAGO");
    }

    #[test]
    fn test_statute_citation() {
        // Implemented states should have citations
        assert_eq!(State::FL.statute_citation(), Some("F.S. Chapter 83"));
        assert_eq!(State::TX.statute_citation(), Some("Tex. Prop. Code Ch. 92"));
        assert_eq!(
            State::CA.statute_citation(),
            Some("CA Civil Code 1940-1954")
        );
        assert_eq!(State::NY.statute_citation(), Some("NY RPL Article 7"));
        assert_eq!(
            State::IL.statute_citation(),
            Some("765 ILCS + Chicago RLTO")
        );

        // All implemented states should have Some citation
        for state in State::implemented_states() {
            assert!(
                state.statute_citation().is_some(),
                "Implemented state {:?} should have statute citation",
                state
            );
        }

        // Unimplemented states should have None
        assert_eq!(State::AL.statute_citation(), None);
        assert_eq!(State::WY.statute_citation(), None);
    }
}
