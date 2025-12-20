use compliance_engine::rules::{attorney_fees, deposit, notices, prohibited};
use serde::{Deserialize, Serialize};
use shared_types::{Severity, Violation};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationItem {
    pub id: String,
    pub violation: Violation,
    pub is_highlighted: bool,
}

#[wasm_bindgen]
pub struct CompliancePanel {
    violations: Vec<ViolationItem>,
    selected_id: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for CompliancePanel {
    fn default() -> Self {
        Self {
            violations: Vec::new(),
            selected_id: None,
        }
    }
}

impl CompliancePanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_violations(&mut self, violations: Vec<Violation>) {
        self.violations = violations
            .into_iter()
            .enumerate()
            .map(|(idx, violation)| ViolationItem {
                id: format!("violation-{}", idx),
                violation,
                is_highlighted: false,
            })
            .collect();
    }

    pub fn violations(&self) -> Vec<&Violation> {
        self.violations.iter().map(|item| &item.violation).collect()
    }

    pub fn filter_by_severity(&self, severity: Severity) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|item| item.violation.severity == severity)
            .map(|item| &item.violation)
            .collect()
    }

    pub fn violations_for_page(&self, page: u32) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|item| item.violation.page == Some(page))
            .map(|item| &item.violation)
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.violations).unwrap_or_default()
    }

    pub fn select_violation(&mut self, id: &str) {
        // Clear previous selection
        for item in &mut self.violations {
            item.is_highlighted = false;
        }

        // Set new selection
        if let Some(item) = self.violations.iter_mut().find(|item| item.id == id) {
            item.is_highlighted = true;
            self.selected_id = Some(id.to_string());
        } else {
            self.selected_id = None;
        }
    }

    pub fn get_selected(&self) -> Option<&ViolationItem> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.violations.iter().find(|item| &item.id == id))
    }

    /// Run comprehensive compliance check combining all rules
    pub fn run_compliance_check(&mut self, text: &str) {
        let mut all_violations = Vec::new();

        // Check prohibited provisions (§ 83.47)
        all_violations.extend(prohibited::check_prohibited_provisions(text));

        // Check security deposit rules (§ 83.49)
        all_violations.extend(deposit::check_security_deposit(text));

        // Check attorney fees clauses (§ 83.49)
        all_violations.extend(attorney_fees::check_attorney_fees(text));

        // Check notice requirements (§ 83.56, § 83.57)
        all_violations.extend(notices::check_notice_requirements(text));

        self.set_violations(all_violations);
    }
}

// WASM bindings
#[wasm_bindgen]
impl CompliancePanel {
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> Self {
        Self::new()
    }

    #[wasm_bindgen(js_name = runComplianceCheck)]
    pub fn run_compliance_check_wasm(&mut self, text: &str) {
        self.run_compliance_check(text);
    }

    #[wasm_bindgen(js_name = getViolationsJson)]
    pub fn get_violations_json(&self) -> String {
        self.to_json()
    }

    #[wasm_bindgen(js_name = getCriticalCount)]
    pub fn get_critical_count(&self) -> u32 {
        self.violations
            .iter()
            .filter(|item| item.violation.severity == Severity::Critical)
            .count() as u32
    }

    #[wasm_bindgen(js_name = getWarningCount)]
    pub fn get_warning_count(&self) -> u32 {
        self.violations
            .iter()
            .filter(|item| item.violation.severity == Severity::Warning)
            .count() as u32
    }

    #[wasm_bindgen(js_name = getInfoCount)]
    pub fn get_info_count(&self) -> u32 {
        self.violations
            .iter()
            .filter(|item| item.violation.severity == Severity::Info)
            .count() as u32
    }

    #[wasm_bindgen(js_name = selectViolation)]
    pub fn select_violation_wasm(&mut self, id: &str) {
        self.select_violation(id);
    }

    #[wasm_bindgen(js_name = getSelectedJson)]
    pub fn get_selected_json(&self) -> Option<String> {
        self.get_selected()
            .and_then(|item| serde_json::to_string(item).ok())
    }

    #[wasm_bindgen(js_name = clearViolations)]
    pub fn clear_violations(&mut self) {
        self.violations.clear();
        self.selected_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::Severity;

    #[test]
    fn test_panel_creation() {
        let panel = CompliancePanel::new();
        assert_eq!(panel.violations().len(), 0);
    }

    #[test]
    fn test_add_violations() {
        let mut panel = CompliancePanel::new();
        let violations = vec![Violation {
            statute: "83.47(1)(a)".to_string(),
            severity: Severity::Critical,
            message: "Waiver of notice detected".to_string(),
            page: Some(1),
            text_snippet: Some("waives right to notice".to_string()),
            text_position: None,
        }];
        panel.set_violations(violations);
        assert_eq!(panel.violations().len(), 1);
    }

    #[test]
    fn test_filter_by_severity() {
        let mut panel = CompliancePanel::new();
        let violations = vec![
            Violation {
                statute: "83.47".to_string(),
                severity: Severity::Critical,
                message: "Critical issue".to_string(),
                page: Some(1),
                text_snippet: None,
                text_position: None,
            },
            Violation {
                statute: "83.49".to_string(),
                severity: Severity::Warning,
                message: "Warning issue".to_string(),
                page: Some(2),
                text_snippet: None,
                text_position: None,
            },
        ];
        panel.set_violations(violations);

        let critical = panel.filter_by_severity(Severity::Critical);
        let warnings = panel.filter_by_severity(Severity::Warning);

        assert_eq!(critical.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_get_violations_for_page() {
        let mut panel = CompliancePanel::new();
        let violations = vec![
            Violation {
                statute: "83.47".to_string(),
                severity: Severity::Critical,
                message: "Page 1 issue".to_string(),
                page: Some(1),
                text_snippet: None,
                text_position: None,
            },
            Violation {
                statute: "83.49".to_string(),
                severity: Severity::Warning,
                message: "Page 2 issue".to_string(),
                page: Some(2),
                text_snippet: None,
                text_position: None,
            },
        ];
        panel.set_violations(violations);

        let page1 = panel.violations_for_page(1);
        assert_eq!(page1.len(), 1);
        assert_eq!(page1[0].statute, "83.47");
    }

    #[test]
    fn test_json_serialization() {
        let mut panel = CompliancePanel::new();
        let violations = vec![Violation {
            statute: "83.47".to_string(),
            severity: Severity::Critical,
            message: "Test".to_string(),
            page: Some(1),
            text_snippet: Some("snippet".to_string()),
            text_position: None,
        }];
        panel.set_violations(violations);

        let json = panel.to_json();
        assert!(json.contains("83.47"));
        assert!(json.contains("Critical"));
    }

    #[test]
    fn test_comprehensive_compliance_check() {
        let mut panel = CompliancePanel::new();

        // Test text with multiple violations from different rule modules
        let lease_text = r#"
            LEASE AGREEMENT

            Tenant hereby waives all rights to notice before termination.

            Security deposit of $1,000 shall be returned within 45 days.

            Tenant shall pay all of Landlord's attorney fees in any dispute.

            Landlord may give 1 day notice for nonpayment of rent.
        "#;

        panel.run_compliance_check(lease_text);

        let violations = panel.violations();

        // Should detect multiple violations
        assert!(
            !violations.is_empty(),
            "Should detect at least one violation"
        );

        // Check that we have violations from different statutes
        let statutes: Vec<String> = violations.iter().map(|v| v.statute.clone()).collect();

        // Should have at least some violations detected
        assert!(
            !statutes.is_empty(),
            "Should detect violations from various statutes"
        );

        // Test severity filtering
        let critical_count = panel.get_critical_count();
        assert!(critical_count > 0, "Should have critical violations");
    }

    #[test]
    fn test_violation_selection() {
        let mut panel = CompliancePanel::new();
        let violations = vec![Violation {
            statute: "83.47".to_string(),
            severity: Severity::Critical,
            message: "Test violation".to_string(),
            page: Some(1),
            text_snippet: None,
            text_position: None,
        }];
        panel.set_violations(violations);

        // Select the first violation
        panel.select_violation("violation-0");

        let selected = panel.get_selected();
        assert!(selected.is_some());
        assert!(selected.unwrap().is_highlighted);
        assert_eq!(selected.unwrap().violation.statute, "83.47");
    }

    #[test]
    fn test_clear_violations() {
        let mut panel = CompliancePanel::new();
        let lease_text = "Tenant waives notice before termination";

        panel.run_compliance_check(lease_text);
        assert!(!panel.violations().is_empty());

        panel.clear_violations();
        assert_eq!(panel.violations().len(), 0);
    }

    #[test]
    fn test_wasm_methods() {
        let mut panel = CompliancePanel::new_wasm();

        let lease_text = "Tenant waives right to notice before eviction";
        panel.run_compliance_check_wasm(lease_text);

        let json = panel.get_violations_json();
        assert!(!json.is_empty());

        let critical_count = panel.get_critical_count();
        let warning_count = panel.get_warning_count();
        let info_count = panel.get_info_count();

        // Should have at least one violation
        assert!(critical_count + warning_count + info_count > 0);
    }
}
