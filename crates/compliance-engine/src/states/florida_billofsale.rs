//! Florida Motor Vehicle Bill of Sale Compliance (Chapter 319)
//!
//! Compliance rules for Florida motor vehicle title transfers including cars,
//! boats, trailers, jet skis, and mobile homes.
//!
//! Key Statutes:
//! - § 319.22 - Transfer of title
//! - § 319.23 - Odometer disclosure requirements
//! - § 319.261 - Mobile home title requirements
//! - § 327.02 - Vessel title requirements

use crate::patterns::extract_snippet;
use regex::Regex;
use shared_types::{Severity, Violation};

/// Document type for Florida bill of sale documents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillOfSaleType {
    /// Motor vehicle (car, truck, motorcycle)
    Car,
    /// Vessel (boat, yacht)
    Boat,
    /// Trailer (utility, travel, cargo)
    Trailer,
    /// Personal watercraft (jet ski, wave runner)
    JetSki,
    /// Mobile home (manufactured housing)
    MobileHome,
    /// Unknown vehicle type
    Unknown,
}

impl BillOfSaleType {
    /// Detect bill of sale type from text content
    pub fn detect(text: &str) -> Self {
        let text_lower = text.to_lowercase();

        // Mobile home - check first as it's most specific
        if text_lower.contains("mobile home")
            || text_lower.contains("manufactured home")
            || text_lower.contains("manufactured housing")
        {
            return Self::MobileHome;
        }

        // Jet ski / personal watercraft
        if text_lower.contains("jet ski")
            || text_lower.contains("jetski")
            || text_lower.contains("personal watercraft")
            || text_lower.contains("wave runner")
            || text_lower.contains("sea-doo")
            || text_lower.contains("pwc")
        {
            return Self::JetSki;
        }

        // Boat / vessel (before trailer since trailers can be for boats)
        if (text_lower.contains("boat") || text_lower.contains("vessel"))
            && (text_lower.contains("hull") || text_lower.contains("registration number"))
        {
            return Self::Boat;
        }

        // Trailer
        if text_lower.contains("trailer")
            && (text_lower.contains("utility")
                || text_lower.contains("travel")
                || text_lower.contains("cargo")
                || text_lower.contains("vin"))
        {
            return Self::Trailer;
        }

        // Car / motor vehicle (default for VIN-containing documents)
        if text_lower.contains("vin")
            || text_lower.contains("vehicle identification")
            || text_lower.contains("odometer")
            || text_lower.contains("motor vehicle")
        {
            return Self::Car;
        }

        Self::Unknown
    }
}

/// Check bill of sale compliance for a given document type
pub fn check_bill_of_sale(text: &str, doc_type: BillOfSaleType) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Common requirements for all bill of sale types
    violations.extend(check_seller_info(text));
    violations.extend(check_buyer_info(text));
    violations.extend(check_sale_price(text));
    violations.extend(check_date_of_sale(text));
    violations.extend(check_signatures(text));

    // Type-specific requirements
    match doc_type {
        BillOfSaleType::Car => {
            violations.extend(check_vin(text));
            violations.extend(check_odometer(text));
            violations.extend(check_vehicle_description(text));
        }
        BillOfSaleType::Boat | BillOfSaleType::JetSki => {
            violations.extend(check_hull_id(text));
            violations.extend(check_vessel_description(text));
        }
        BillOfSaleType::Trailer => {
            violations.extend(check_vin(text));
            violations.extend(check_trailer_description(text));
        }
        BillOfSaleType::MobileHome => {
            violations.extend(check_vin(text)); // Mobile homes have VINs
            violations.extend(check_mobile_home_description(text));
        }
        BillOfSaleType::Unknown => {
            // Try to detect and check VIN anyway
            violations.extend(check_vin(text));
        }
    }

    violations
}

/// Check for seller information
fn check_seller_info(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for seller name
    if !text_lower.contains("seller")
        && !text_lower.contains("owner")
        && !text_lower.contains("sold by")
        && !text_lower.contains("transferor")
    {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Critical,
            message: "Missing seller information - include seller's full legal name and address"
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for buyer information
fn check_buyer_info(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for buyer name
    if !text_lower.contains("buyer")
        && !text_lower.contains("purchaser")
        && !text_lower.contains("transferee")
        && !text_lower.contains("sold to")
    {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Critical,
            message: "Missing buyer information - include buyer's full legal name and address"
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for sale price
fn check_sale_price(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Look for price indicators
    let has_price = text_lower.contains("price")
        || text_lower.contains("amount")
        || text_lower.contains("consideration")
        || text_lower.contains("purchase")
        || contains_dollar_amount(text);

    if !has_price {
        violations.push(Violation {
            statute: "F.S. § 319.22(2)".to_string(),
            severity: Severity::Warning,
            message: "Missing sale price - if gift, state 'gift' or '$0 (gift)'".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for date of sale
fn check_date_of_sale(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Look for date patterns
    let date_pattern = Regex::new(r"\d{1,2}/\d{1,2}/\d{2,4}|\d{1,2}-\d{1,2}-\d{2,4}").unwrap();
    let has_date = date_pattern.is_match(text)
        || text.to_lowercase().contains("date of sale")
        || text.to_lowercase().contains("sale date")
        || text.to_lowercase().contains("dated");

    if !has_date {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Warning,
            message: "Missing date of sale - include date in MM/DD/YYYY format".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for signatures
fn check_signatures(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for signature indicators
    let has_signature = text_lower.contains("signature")
        || text_lower.contains("signed")
        || text_lower.contains("sign here")
        || text_lower.contains("x__");

    if !has_signature {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Critical,
            message: "Missing signature lines for seller and buyer".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for VIN (Vehicle Identification Number)
fn check_vin(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for VIN reference
    let has_vin_field =
        text_lower.contains("vin") || text_lower.contains("vehicle identification number");

    if !has_vin_field {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)(a)".to_string(),
            severity: Severity::Critical,
            message:
                "Missing Vehicle Identification Number (VIN) - include complete 17-character VIN"
                    .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    // Check for VIN format (17 alphanumeric characters, excluding I, O, Q)
    let vin_pattern = Regex::new(r"[A-HJ-NPR-Z0-9]{17}").unwrap();
    if has_vin_field && !vin_pattern.is_match(&text.to_uppercase()) {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)(a)".to_string(),
            severity: Severity::Warning,
            message: "VIN format may be incorrect - must be exactly 17 characters".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "vin")),
            text_position: None,
        });
    }

    violations
}

/// Check for odometer disclosure (required for cars under 49 CFR 580)
fn check_odometer(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for odometer disclosure
    if !text_lower.contains("odometer")
        && !text_lower.contains("mileage")
        && !text_lower.contains("miles")
    {
        violations.push(Violation {
            statute: "F.S. § 319.23".to_string(),
            severity: Severity::Critical,
            message: "Missing odometer disclosure - federal law requires mileage statement"
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for vehicle description (year, make, model)
fn check_vehicle_description(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Check for year
    let year_pattern = Regex::new(r"\b(19|20)\d{2}\b").unwrap();
    if !year_pattern.is_match(text) {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Warning,
            message: "Vehicle year may be missing".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for hull identification number (boats/PWC)
fn check_hull_id(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for hull ID
    if !text_lower.contains("hull")
        && !text_lower.contains("hin")
        && !text_lower.contains("registration number")
    {
        violations.push(Violation {
            statute: "F.S. § 327.02".to_string(),
            severity: Severity::Critical,
            message: "Missing Hull Identification Number (HIN) - include 12-character HIN"
                .to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for vessel description
fn check_vessel_description(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for length
    if !text_lower.contains("length") && !text_lower.contains("feet") && !text_lower.contains("ft")
    {
        violations.push(Violation {
            statute: "F.S. § 327.02".to_string(),
            severity: Severity::Warning,
            message: "Vessel length may be missing".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for trailer description
fn check_trailer_description(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for trailer type
    let trailer_types = [
        "utility",
        "travel",
        "cargo",
        "enclosed",
        "flatbed",
        "boat trailer",
        "horse trailer",
    ];
    let has_type =
        trailer_types.iter().any(|t| text_lower.contains(t)) || text_lower.contains("type:");

    if !has_type {
        violations.push(Violation {
            statute: "F.S. § 319.22(1)".to_string(),
            severity: Severity::Info,
            message: "Consider specifying trailer type (utility, travel, cargo)".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Check for mobile home description
fn check_mobile_home_description(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for serial number / label number
    if !text_lower.contains("serial")
        && !text_lower.contains("label number")
        && !text_lower.contains("hud label")
    {
        violations.push(Violation {
            statute: "F.S. § 319.261".to_string(),
            severity: Severity::Critical,
            message: "Missing mobile home serial/HUD label number".to_string(),
            page: None,
            text_snippet: None,
            text_position: None,
        });
    }

    violations
}

/// Helper to check if text contains a dollar amount
fn contains_dollar_amount(text: &str) -> bool {
    let dollar_pattern = Regex::new(r"\$\s*[\d,]+(\.\d{2})?").unwrap();
    dollar_pattern.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_car_bill_of_sale() {
        let text = "Bill of Sale\nVIN: 1HGBH41JXMN109186\nOdometer: 45,000 miles";
        assert_eq!(BillOfSaleType::detect(text), BillOfSaleType::Car);
    }

    #[test]
    fn test_detect_boat_bill_of_sale() {
        let text = "Vessel Bill of Sale\nHull ID: ABC12345D607\nRegistration Number: FL1234AB";
        assert_eq!(BillOfSaleType::detect(text), BillOfSaleType::Boat);
    }

    #[test]
    fn test_detect_mobile_home() {
        let text = "Mobile Home Bill of Sale\nManufactured Housing\nSerial Number: ABC123";
        assert_eq!(BillOfSaleType::detect(text), BillOfSaleType::MobileHome);
    }

    #[test]
    fn test_detect_jetski() {
        let text = "Personal Watercraft Bill of Sale\nJet Ski Sale Agreement";
        assert_eq!(BillOfSaleType::detect(text), BillOfSaleType::JetSki);
    }

    #[test]
    fn test_missing_vin() {
        let text = "Bill of Sale\nSeller: John Doe\nBuyer: Jane Smith";
        let violations = check_vin(text);
        assert!(violations.iter().any(|v| v.message.contains("VIN")));
    }

    #[test]
    fn test_has_vin() {
        let text = "Bill of Sale\nVIN: 1HGBH41JXMN109186";
        let violations = check_vin(text);
        // Should not have "missing VIN" violation
        assert!(!violations
            .iter()
            .any(|v| v.message.contains("Missing Vehicle Identification Number")));
    }

    #[test]
    fn test_missing_odometer() {
        let text = "Car Bill of Sale\nVIN: 1HGBH41JXMN109186";
        let violations = check_odometer(text);
        assert!(violations.iter().any(|v| v.message.contains("odometer")));
    }

    #[test]
    fn test_complete_car_bill_of_sale() {
        let text = r#"
            FLORIDA MOTOR VEHICLE BILL OF SALE

            Seller: John Doe
            Seller Address: 123 Main St, Miami, FL 33101

            Buyer: Jane Smith
            Buyer Address: 456 Oak Ave, Tampa, FL 33602

            Vehicle: 2020 Toyota Camry
            VIN: 1HGBH41JXMN109186
            Odometer: 45,000 miles (actual mileage)

            Sale Price: $25,000.00
            Date of Sale: 01/15/2025

            Seller Signature: X_______________
            Buyer Signature: X_______________
        "#;

        let violations = check_bill_of_sale(text, BillOfSaleType::Car);

        // Should have minimal violations
        let critical_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.severity == Severity::Critical)
            .collect();
        assert!(
            critical_violations.is_empty(),
            "Complete bill of sale should have no critical violations: {:?}",
            critical_violations
        );
    }

    #[test]
    fn test_boat_hull_id_required() {
        let text = "Boat Bill of Sale\nVessel: 2020 Sea Ray 240";
        let violations = check_hull_id(text);
        assert!(violations
            .iter()
            .any(|v| v.message.contains("Hull Identification Number")));
    }

    #[test]
    fn test_dollar_amount_detection() {
        assert!(contains_dollar_amount("Price: $1,500.00"));
        assert!(contains_dollar_amount("$500"));
        assert!(!contains_dollar_amount("Five hundred dollars"));
    }
}
