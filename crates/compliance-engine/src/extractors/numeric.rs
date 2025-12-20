// Numeric extraction utilities for compliance validation
use regex::Regex;

/// Extracts a number that appears near keywords related to deposit returns
/// Returns the number of days if found
pub fn extract_days_near_deposit_return(text: &str) -> Option<u32> {
    let text_lower = text.to_lowercase();

    // Look for patterns like "within X days" or "X days" near "return" and "deposit"
    let re = Regex::new(r"(?:within\s+)?(\d+)\s+days?").unwrap();

    // Find all day references
    for cap in re.captures_iter(&text_lower) {
        if let Some(num_match) = cap.get(1) {
            if let Ok(days) = num_match.as_str().parse::<u32>() {
                // Check if this appears in context of deposit return
                let start = cap.get(0).unwrap().start();
                let context_start = start.saturating_sub(50);
                let context_end = (start + 50).min(text_lower.len());
                let context = &text_lower[context_start..context_end];

                if (context.contains("return") || context.contains("refund"))
                    && context.contains("deposit")
                {
                    return Some(days);
                }
            }
        }
    }

    None
}

/// Checks if text mentions a claim or intent to claim
pub fn has_claim_context(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    text_lower.contains("claim")
        || text_lower.contains("impose")
        || text_lower.contains("deduction")
        || text_lower.contains("withhold")
}

/// Checks if text mentions bank location information
pub fn has_bank_location(text: &str) -> bool {
    let text_lower = text.to_lowercase();

    // Check for bank mention
    let has_bank = text_lower.contains("bank")
        || text_lower.contains("credit union")
        || text_lower.contains("financial institution");

    if !has_bank {
        return false;
    }

    // Check for location indicators (city names or "florida")
    let has_location = text_lower.contains("florida") ||
                       text_lower.contains("miami") ||
                       text_lower.contains("tampa") ||
                       text_lower.contains("orlando") ||
                       text_lower.contains("jacksonville") ||
                       // Generic location pattern indicators
                       text_lower.contains(", fl") ||
                       // Check for address-like patterns near bank
                       text_lower.matches(",").count() >= 2;

    has_location
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_days_near_deposit_return() {
        assert_eq!(
            extract_days_near_deposit_return("Landlord shall return deposit within 45 days"),
            Some(45)
        );
        assert_eq!(
            extract_days_near_deposit_return("Deposit will be returned within 15 days"),
            Some(15)
        );
        assert_eq!(
            extract_days_near_deposit_return("Notice within 30 days"),
            None // No "return" or "deposit" in close context
        );
    }

    #[test]
    fn test_has_claim_context() {
        assert!(has_claim_context("If landlord intends to impose a claim"));
        assert!(has_claim_context("making deductions from deposit"));
        assert!(!has_claim_context("return deposit within 15 days"));
    }

    #[test]
    fn test_has_bank_location() {
        assert!(has_bank_location("First National Bank, Miami, Florida"));
        assert!(has_bank_location("held at SunTrust Bank in Tampa"));
        assert!(!has_bank_location("deposit held at bank"));
    }
}
