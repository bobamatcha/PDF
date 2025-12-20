use crate::patterns::{
    contains_semantic_cluster, extract_snippet, find_text_position, AS_IS_KEYWORDS,
    DISPOSAL_KEYWORDS, FL_LAW_KEYWORDS, NOTICE_KEYWORDS, PROPERTY_KEYWORDS, RIGHTS_KEYWORDS,
    STRUCTURAL_KEYWORDS, TENANT_KEYWORDS, TERMINATION_KEYWORDS, WAIVER_KEYWORDS,
};
use shared_types::{Severity, TextPosition, Violation};

/// Check for prohibited provisions under Florida Statute § 83.47
pub fn check_prohibited_provisions(text: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    // Check for waiver of notice (§ 83.47(1)(c))
    if check_waiver_of_notice(&text_lower) {
        let text_position = find_text_position(text, "waive").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "83.47(1)(a)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited waiver of tenant's right to notice before termination or eviction".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "waive")),
            text_position,
        });
    }

    // Check for property disposal clause (§ 83.47(1)(b))
    if check_property_disposal(&text_lower) {
        let text_position = find_text_position(text, "dispose").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "83.47(1)(b)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited authorization for landlord to dispose of tenant's property".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "dispose")),
            text_position,
        });
    }

    // Check for AS-IS clauses that waive landlord's structural obligations (§ 83.51)
    if check_as_is_structural(&text_lower) {
        let text_position = find_text_position(text, "as-is")
            .or_else(|| find_text_position(text, "as is"))
            .map(|(start, end)| TextPosition {
                start_offset: start,
                end_offset: end,
            });
        violations.push(Violation {
            statute: "83.51(2)(a)".to_string(),
            severity: Severity::Critical,
            message: "AS-IS clause may improperly waive landlord's obligation to maintain structural components".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "as-is")),
            text_position,
        });
    }

    // Check for general waiver of rights (§ 83.47(1)(a))
    if check_general_rights_waiver(&text_lower) {
        let text_position = find_text_position(text, "waive").map(|(start, end)| TextPosition {
            start_offset: start,
            end_offset: end,
        });
        violations.push(Violation {
            statute: "83.47(1)(a)".to_string(),
            severity: Severity::Critical,
            message: "Lease contains prohibited waiver of tenant's rights under Florida landlord-tenant law".to_string(),
            page: None,
            text_snippet: Some(extract_snippet(text, "waive")),
            text_position,
        });
    }

    violations
}

/// Detect waiver of notice before termination/eviction
fn check_waiver_of_notice(text: &str) -> bool {
    contains_semantic_cluster(
        text,
        &[WAIVER_KEYWORDS, NOTICE_KEYWORDS, TERMINATION_KEYWORDS],
    )
}

/// Detect authorization for landlord to dispose of tenant's property
fn check_property_disposal(text: &str) -> bool {
    // Must have disposal keyword + property keyword + context indicating tenant
    let has_disposal = DISPOSAL_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_property = PROPERTY_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_tenant_context =
        TENANT_KEYWORDS.iter().any(|kw| text.contains(kw)) || text.contains("left by");

    has_disposal && has_property && has_tenant_context
}

/// Detect AS-IS clauses combined with structural maintenance items
fn check_as_is_structural(text: &str) -> bool {
    let has_as_is = AS_IS_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_structural = STRUCTURAL_KEYWORDS.iter().any(|kw| text.contains(kw));

    has_as_is && has_structural
}

/// Detect general waiver of tenant rights under Florida law
fn check_general_rights_waiver(text: &str) -> bool {
    // Look for waiver + rights + Florida law reference
    let has_waiver = WAIVER_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_rights = RIGHTS_KEYWORDS.iter().any(|kw| text.contains(kw));
    let has_law_ref = FL_LAW_KEYWORDS.iter().any(|kw| text.contains(kw));

    // Must have waiver + rights, and ideally law reference
    has_waiver && has_rights && (has_law_ref || text.contains("all"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_waiver_of_notice() {
        let text = "Tenant hereby waives any right to notice before termination";
        let violations = check_prohibited_provisions(text);
        assert!(violations.iter().any(|v| v.statute == "83.47(1)(a)"));
        assert!(violations.iter().any(|v| v.severity == Severity::Critical));
    }

    #[test]
    fn test_detects_property_disposal_clause() {
        let text = "Landlord may dispose of any property left by tenant after 24 hours";
        let violations = check_prohibited_provisions(text);
        assert!(violations.iter().any(|v| v.statute == "83.47(1)(b)"));
    }

    #[test]
    fn test_detects_as_is_for_structural() {
        let text = "Tenant accepts property AS-IS and is responsible for all plumbing repairs";
        let violations = check_prohibited_provisions(text);
        assert!(violations.iter().any(|v| v.statute.contains("83.51")));
    }

    #[test]
    fn test_allows_valid_clauses() {
        let text = "Tenant shall maintain the lawn in good condition";
        let violations = check_prohibited_provisions(text);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_detects_rights_waiver() {
        let text = "Tenant waives all rights under Florida landlord tenant law";
        let violations = check_prohibited_provisions(text);
        assert!(!violations.is_empty());
    }
}
