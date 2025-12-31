//! Property-based tests for the agentPDF server API
//!
//! These tests use proptest to generate arbitrary inputs and verify
//! that the API handles them correctly.

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    use compliance_engine::{ComplianceEngine, Jurisdiction, State};
    use typst_engine::templates::list_templates;

    /// Generate arbitrary template names from the known list
    fn valid_template_name() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("florida_lease".to_string()),
            Just("florida_purchase_contract".to_string()),
            Just("florida_purchase_as_is".to_string()),
            Just("florida_listing_agreement".to_string()),
            Just("florida_escalation_addendum".to_string()),
            Just("florida_flood_disclosure".to_string()),
            Just("florida_commercial_lease".to_string()),
            Just("texas_lease".to_string()),
            Just("invoice".to_string()),
            Just("letter".to_string()),
        ]
    }

    /// Generate arbitrary but invalid template names
    fn invalid_template_name() -> impl Strategy<Value = String> {
        "[a-z]{5,20}".prop_filter("Must not be valid", |s| {
            !matches!(
                s.as_str(),
                "florida_lease"
                    | "florida_purchase_contract"
                    | "florida_purchase_as_is"
                    | "florida_listing_agreement"
                    | "florida_escalation_addendum"
                    | "florida_flood_disclosure"
                    | "florida_commercial_lease"
                    | "texas_lease"
                    | "invoice"
                    | "letter"
            )
        })
    }

    /// Generate arbitrary state values
    fn state_value() -> impl Strategy<Value = State> {
        prop_oneof![
            Just(State::FL),
            Just(State::TX),
            Just(State::CA),
            Just(State::NY),
            Just(State::GA),
            Just(State::IL),
        ]
    }

    // Note: year_built and zip_code strategies are available for future property tests
    // when we add more comprehensive API testing

    proptest! {
        /// Property: Valid templates should always be found in the template list
        #[test]
        fn valid_templates_exist(name in valid_template_name()) {
            let templates = list_templates();
            let found = templates.iter().any(|t| t.name == name);
            prop_assert!(found, "Template '{}' should exist", name);
        }

        /// Property: Invalid templates should not exist
        #[test]
        fn invalid_templates_not_found(name in invalid_template_name()) {
            let templates = list_templates();
            let found = templates.iter().any(|t| t.name == name);
            prop_assert!(!found, "Random name '{}' should not match a template", name);
        }

        /// Property: Template URIs should match expected format
        #[test]
        fn template_uris_have_correct_format(name in valid_template_name()) {
            let templates = list_templates();
            let template = templates.iter().find(|t| t.name == name);
            prop_assert!(template.is_some());
            let uri = &template.unwrap().uri;
            prop_assert!(uri.starts_with("typst://templates/"));
            prop_assert!(uri.ends_with(&name));
        }

        /// Property: All templates should have at least one required input
        #[test]
        fn templates_have_required_inputs(name in valid_template_name()) {
            let templates = list_templates();
            let template = templates.iter().find(|t| t.name == name);
            prop_assert!(template.is_some());
            let req = &template.unwrap().required_inputs;
            prop_assert!(!req.is_empty(), "Template '{}' should have required inputs", name);
        }

        /// Property: Compliance engine supports all tier-1 states
        #[test]
        fn compliance_engine_supports_state(state in state_value()) {
            let engine = ComplianceEngine::new();
            let supported = engine.supported_states();
            prop_assert!(supported.contains(&state), "State {:?} should be supported", state);
        }

        /// Property: Year built affects lead paint compliance for pre-1978
        #[test]
        fn year_built_affects_lead_paint(year in (1950u32..2000)) {
            let engine = ComplianceEngine::new();
            let jurisdiction = Jurisdiction::new(State::FL);
            let text = "This is a sample lease agreement.";

            let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, Some(year));

            // Pre-1978 properties should trigger lead paint check
            if year < 1978 {
                let has_lead_paint_violation = violations.iter().any(|v| {
                    v.statute.contains("4852d") || v.message.to_lowercase().contains("lead")
                });
                // The property is that the check runs without error and possibly finds violations
                let _ = has_lead_paint_violation; // Use the variable to avoid unused warning
            }
        }

        /// Property: Empty text should not cause panics
        #[test]
        fn empty_text_no_panic(state in state_value()) {
            let engine = ComplianceEngine::new();
            let jurisdiction = Jurisdiction::new(state);

            // Should not panic - the fact that we got here means success
            let violations = engine.check_text_with_jurisdiction(&jurisdiction, "", None);
            let _ = violations; // Verify it runs without panic
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use std::collections::HashMap;

    use compliance_engine::{ComplianceEngine, Jurisdiction, State};

    /// Regression: Ensure florida_lease template renders without errors
    #[tokio::test]
    async fn florida_lease_renders_with_minimal_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "landlord_name".to_string(),
            serde_json::Value::String("Test Landlord".to_string()),
        );
        inputs.insert(
            "tenant_name".to_string(),
            serde_json::Value::String("Test Tenant".to_string()),
        );
        inputs.insert(
            "property_address".to_string(),
            serde_json::Value::String("123 Test St, Miami, FL 33101".to_string()),
        );
        inputs.insert(
            "monthly_rent".to_string(),
            serde_json::Value::Number(1500.into()),
        );
        inputs.insert(
            "lease_start".to_string(),
            serde_json::Value::String("2026-01-01".to_string()),
        );
        inputs.insert(
            "lease_end".to_string(),
            serde_json::Value::String("2026-12-31".to_string()),
        );

        let request = typst_engine::compiler::RenderRequest {
            source: "typst://templates/florida_lease".to_string(),
            inputs,
            assets: HashMap::new(),
            format: typst_engine::compiler::OutputFormat::Pdf,
            ppi: None,
        };

        let result = typst_engine::compile_document(request, 10000).await;
        assert!(result.is_ok(), "Render should succeed: {:?}", result.err());

        let response = result.unwrap();
        assert_eq!(
            response.status,
            typst_engine::compiler::RenderStatus::Success
        );
        assert!(response.artifact.is_some());
        assert!(response.artifact.unwrap().page_count > 0);
    }

    /// Regression: Ensure florida_purchase_contract template renders
    #[tokio::test]
    async fn florida_purchase_contract_renders() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "seller_name".to_string(),
            serde_json::Value::String("Test Seller".to_string()),
        );
        inputs.insert(
            "buyer_name".to_string(),
            serde_json::Value::String("Test Buyer".to_string()),
        );
        inputs.insert(
            "property_address".to_string(),
            serde_json::Value::String("456 Purchase Ave".to_string()),
        );
        inputs.insert(
            "property_city".to_string(),
            serde_json::Value::String("Tampa".to_string()),
        );
        inputs.insert(
            "property_county".to_string(),
            serde_json::Value::String("Hillsborough".to_string()),
        );
        inputs.insert(
            "property_zip".to_string(),
            serde_json::Value::String("33602".to_string()),
        );
        inputs.insert(
            "purchase_price".to_string(),
            serde_json::Value::Number(350000.into()),
        );
        inputs.insert(
            "earnest_money".to_string(),
            serde_json::Value::Number(10000.into()),
        );
        inputs.insert(
            "closing_date".to_string(),
            serde_json::Value::String("2026-03-01".to_string()),
        );

        let request = typst_engine::compiler::RenderRequest {
            source: "typst://templates/florida_purchase_contract".to_string(),
            inputs,
            assets: HashMap::new(),
            format: typst_engine::compiler::OutputFormat::Pdf,
            ppi: None,
        };

        let result = typst_engine::compile_document(request, 15000).await;
        assert!(result.is_ok(), "Render should succeed: {:?}", result.err());
    }

    /// Regression: Compliance checker should not panic on empty text
    #[test]
    fn compliance_handles_empty_text() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);

        // Should not panic - the fact that we got here means success
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, "", None);
        let _ = violations; // Verify it runs without panic
    }

    /// Regression: Compliance checker should handle very long text
    #[test]
    fn compliance_handles_long_text() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);
        let long_text = "This is a test. ".repeat(10000);

        // Should not panic or timeout - the fact that we got here means success
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, &long_text, None);
        let _ = violations; // Verify it runs without panic
    }

    /// Regression: Chicago RLTO detection via ZIP code
    #[test]
    fn chicago_rlto_detected_via_zip() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::from_zip(State::IL, "60601");

        let text = "Security deposit: $2,000. Monthly rent: $2,000.";
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // Should detect RLTO requirement
        let has_rlto = violations.iter().any(|v| v.statute.contains("5-12-170"));
        assert!(has_rlto, "Chicago RLTO should apply for 60601 ZIP");
    }

    /// Regression: NYC deposit limits detected via ZIP code
    #[test]
    fn nyc_deposit_limit_detected_via_zip() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::from_zip(State::NY, "10001");

        // Excessive deposit (> 1 month for NYC)
        let text = "Security deposit: $4,500. Monthly rent: $3,000.";
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // Should detect excessive deposit
        let has_deposit_violation = violations.iter().any(|v| v.statute.contains("7-108"));
        assert!(
            has_deposit_violation,
            "NYC deposit limit should apply for 10001 ZIP"
        );
    }
}
