use shared_types::{Severity, Violation};

/// Checks attorney fee clauses for reciprocity per Florida Statute § 83.48
///
/// Florida Statute § 83.48 requires that attorney fee clauses be reciprocal.
/// If the lease allows the landlord to recover attorney fees, the tenant must
/// have the same right. "Prevailing party" clauses are compliant. One-sided
/// fee clauses are unenforceable.
pub fn check_attorney_fees(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for compliant reciprocal clauses first
    let has_prevailing_party = text_lower.contains("prevailing party");
    let has_both_parties = text_lower.contains("both parties")
        || text_lower.contains("either party")
        || (text_lower.contains("both")
            && text_lower.contains("landlord")
            && text_lower.contains("tenant"));

    // If it has a compliant reciprocal clause, it's fine
    if has_prevailing_party || has_both_parties {
        return violations;
    }

    // Check for attorney fee mentions
    let has_attorney_fees = text_lower.contains("attorney")
        && (text_lower.contains("fee") || text_lower.contains("cost"));

    if !has_attorney_fees {
        return violations; // No attorney fee clause, no problem
    }

    // Check for landlord rights to attorney fees (looking for patterns in proximity)
    let has_landlord_fee_right = text_lower.contains("landlord is entitled to attorney fee")
        || text_lower.contains("landlord shall be entitled to attorney fee")
        || text_lower.contains("lessor is entitled to attorney fee")
        || text_lower.contains("landlord is entitled to recover attorney fee")
        || text_lower.contains("landlord may recover attorney fee");

    // Check if tenant has obligation to pay landlord's fees/costs
    let has_tenant_pay_obligation = text_lower.contains("tenant shall pay")
        && (text_lower.contains("landlord") || text_lower.contains("attorney"))
        || text_lower.contains("tenant agrees to pay")
            && (text_lower.contains("landlord")
                || text_lower.contains("attorney")
                || text_lower.contains("legal"));

    // Check if tenant has reciprocal rights (looking for tenant-specific entitlements)
    let has_tenant_fee_right = text_lower.contains("tenant is entitled to attorney fee")
        || text_lower.contains("tenant shall be entitled to attorney fee")
        || text_lower.contains("lessee is entitled to attorney fee")
        || text_lower.contains("tenant is entitled to recover attorney fee")
        || text_lower.contains("tenant may recover attorney fee");

    // If landlord has fee rights or tenant must pay, but tenant doesn't have equal rights
    if (has_landlord_fee_right || has_tenant_pay_obligation) && !has_tenant_fee_right {
        violations.push(Violation {
            statute: "83.48".to_string(),
            severity: Severity::Critical,
            message: "Attorney fee clause is not reciprocal. Florida Statute § 83.48 requires that if the landlord can recover attorney fees, the tenant must have the same right. Use 'prevailing party' language or ensure mutual fee recovery rights.".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text)),
            text_position: None,
        });
    }

    violations
}

/// Extracts a relevant snippet of text (limited to 200 chars)
fn extract_snippet(text: &str) -> String {
    if text.len() <= 200 {
        text.to_string()
    } else {
        format!("{}...", &text[..200])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Severity;

    #[test]
    fn test_flags_non_reciprocal_fees() {
        let text = "Tenant shall pay all landlord's attorney fees in any dispute";
        let violations = check_attorney_fees(text);
        assert!(violations.iter().any(|v| v.statute == "83.48"));
        assert!(violations.iter().any(|v| v.severity == Severity::Critical));
    }

    #[test]
    fn test_accepts_prevailing_party_clause() {
        let text = "The prevailing party in any legal action shall be entitled to reasonable attorney fees";
        let violations = check_attorney_fees(text);
        let fee_violations: Vec<_> = violations.iter().filter(|v| v.statute == "83.48").collect();
        assert!(fee_violations.is_empty());
    }

    #[test]
    fn test_accepts_mutual_fees_clause() {
        let text = "Both landlord and tenant shall be entitled to recover attorney fees if they prevail in court";
        let violations = check_attorney_fees(text);
        let fee_violations: Vec<_> = violations.iter().filter(|v| v.statute == "83.48").collect();
        assert!(fee_violations.is_empty());
    }

    #[test]
    fn test_flags_landlord_only_fees() {
        let text = "Landlord is entitled to attorney fees. Tenant agrees to pay all legal costs.";
        let violations = check_attorney_fees(text);
        assert!(violations
            .iter()
            .any(|v| v.statute == "83.48" && v.message.contains("reciprocal")));
    }

    #[test]
    fn test_no_fee_clause_is_fine() {
        let text =
            "This lease agreement is between landlord and tenant for the property at 123 Main St.";
        let violations = check_attorney_fees(text);
        let fee_violations: Vec<_> = violations.iter().filter(|v| v.statute == "83.48").collect();
        assert!(fee_violations.is_empty());
    }
}
