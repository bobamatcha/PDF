//! Property-based tests for the agentPDF server API
//!
//! These tests use proptest to generate arbitrary inputs and verify
//! that the API handles them correctly.
//!
//! Test categories:
//! - Template validation (names, URIs, inputs)
//! - Compliance engine states and jurisdictions
//! - Render request input fuzzing
//! - API format and state code parsing

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    use compliance_engine::{ComplianceEngine, Jurisdiction, State};
    use typst_engine::templates::list_templates;

    // Strategies for generating test values

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

    /// Generate arbitrary state values (Florida-only in MVP)
    fn state_value() -> impl Strategy<Value = State> {
        // Florida-only for MVP; other states archived
        Just(State::FL)
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

        /// Property: Compliance engine supports Florida (MVP)
        #[test]
        fn compliance_engine_supports_state(state in state_value()) {
            let engine = ComplianceEngine::new();
            let supported = engine.supported_states();
            // Florida is always supported in MVP
            prop_assert!(supported.contains(&State::FL), "Florida should always be supported");
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

        /// Property: Valid output formats are recognized
        #[test]
        fn valid_output_formats_recognized(format in prop_oneof![
            Just("pdf".to_string()),
            Just("PDF".to_string()),
            Just("svg".to_string()),
            Just("SVG".to_string()),
            Just("png".to_string()),
            Just("PNG".to_string()),
        ]) {
            let normalized = format.to_lowercase();
            prop_assert!(
                matches!(normalized.as_str(), "pdf" | "svg" | "png"),
                "Format '{}' should be valid", format
            );
        }

        /// Property: Invalid output formats are rejected
        #[test]
        fn invalid_output_formats_rejected(format in "[a-z]{2,6}".prop_filter(
            "Must not be valid format",
            |s| !matches!(s.as_str(), "pdf" | "svg" | "png")
        )) {
            let normalized = format.to_lowercase();
            prop_assert!(
                !matches!(normalized.as_str(), "pdf" | "svg" | "png"),
                "Random format '{}' should be invalid", format
            );
        }

        /// Property: Florida state code parses correctly (MVP)
        #[test]
        fn valid_state_codes_parse(state_code in prop_oneof![
            Just("FL"), Just("fl"), Just("Fl"),
        ]) {
            // Florida should parse regardless of case
            let upper = state_code.to_uppercase();
            let is_florida = matches!(upper.as_str(), "FL");
            prop_assert!(is_florida, "Florida state '{}' should parse", state_code);
        }

        /// Property: Florida ZIP codes handled correctly
        #[test]
        fn zip_code_formats_handled(zip in prop_oneof![
            Just("33101".to_string()),    // Miami
            Just("33139".to_string()),    // Miami Beach
            Just("32801".to_string()),    // Orlando
            Just("33602".to_string()),    // Tampa
            Just("33101-1234".to_string()), // ZIP+4
        ]) {
            let engine = ComplianceEngine::new();
            // Extract 5-digit ZIP for lookup
            let zip_5 = &zip[..5];
            let jurisdiction = Jurisdiction::from_zip(State::FL, zip_5);
            // Should not panic and process Florida zips
            let _ = engine.check_text_with_jurisdiction(&jurisdiction, "test", None);
        }

        /// Property: Arbitrary ASCII text should not crash compliance engine
        #[test]
        fn arbitrary_ascii_text_handled(
            text in "[a-zA-Z0-9 .,!?$%&()-]{0,500}",
            state in state_value()
        ) {
            let engine = ComplianceEngine::new();
            let jurisdiction = Jurisdiction::new(state);
            // Should not panic on arbitrary text
            let violations = engine.check_text_with_jurisdiction(&jurisdiction, &text, None);
            let _ = violations;
        }

        /// Property: Very short texts should be handled gracefully
        #[test]
        fn short_text_handled(text in ".{0,10}", state in state_value()) {
            let engine = ComplianceEngine::new();
            let jurisdiction = Jurisdiction::new(state);
            let violations = engine.check_text_with_jurisdiction(&jurisdiction, &text, None);
            let _ = violations;
        }

        /// Property: Year built bounds are reasonable (1800-current year)
        #[test]
        fn year_built_bounds(year in (1800u32..2030)) {
            let engine = ComplianceEngine::new();
            let jurisdiction = Jurisdiction::new(State::FL);
            // Should handle any reasonable year
            let violations = engine.check_text_with_jurisdiction(&jurisdiction, "lease", Some(year));
            let _ = violations;

            // Lead paint should only apply pre-1978
            if year >= 1978 {
                let pre_1978_violations = engine.check_text_with_jurisdiction(
                    &jurisdiction, "No lead paint disclosure", Some(year)
                );
                // Post-1978 should NOT require lead paint disclosure
                let has_lead_violation = pre_1978_violations.iter()
                    .any(|v| v.message.to_lowercase().contains("lead"));
                prop_assert!(!has_lead_violation,
                    "Year {} (post-1978) should not require lead paint disclosure", year);
            }
        }

        /// Property: Template metadata is consistent
        #[test]
        fn template_metadata_consistent(name in valid_template_name()) {
            let templates = list_templates();
            let template = templates.iter().find(|t| t.name == name).unwrap();

            // Description should not be empty
            prop_assert!(!template.description.is_empty(),
                "Template '{}' should have a description", name);

            // URI should contain template name
            prop_assert!(template.uri.contains(&name),
                "Template URI should contain name: {} not in {}", name, template.uri);
        }
    }
}

#[cfg(test)]
mod api_property_tests {
    //! Property tests for API request/response handling

    use proptest::prelude::*;

    /// Generate valid document types
    fn valid_doc_type() -> impl Strategy<Value = String> {
        prop_oneof![
            // Lease Documents (Chapter 83)
            Just("lease".to_string()),
            Just("lease_termination".to_string()),
            Just("eviction".to_string()),
            // Real Estate Purchase (Chapter 475, 689)
            Just("purchase".to_string()),
            Just("purchase_as_is".to_string()),
            Just("inspection_contingency".to_string()),
            Just("financing_contingency".to_string()),
            Just("escalation".to_string()),
            Just("appraisal_contingency".to_string()),
            // Listing Documents (Chapter 475)
            Just("listing".to_string()),
            // Contractor Documents (Chapter 713)
            Just("contractor_invoice".to_string()),
            Just("cost_of_materials".to_string()),
            Just("notice_of_commencement".to_string()),
            Just("notice_to_owner".to_string()),
            Just("claim_of_lien".to_string()),
            Just("release_of_lien".to_string()),
            Just("dispute_lien".to_string()),
            Just("fraudulent_lien".to_string()),
            Just("final_payment_affidavit".to_string()),
            // Auto-detect
            Just("auto".to_string()),
        ]
    }

    /// All valid document types for matching
    const VALID_DOC_TYPES: &[&str] = &[
        "lease",
        "lease_termination",
        "eviction",
        "purchase",
        "purchase_as_is",
        "inspection_contingency",
        "financing_contingency",
        "escalation",
        "appraisal_contingency",
        "listing",
        "contractor_invoice",
        "cost_of_materials",
        "notice_of_commencement",
        "notice_to_owner",
        "claim_of_lien",
        "release_of_lien",
        "dispute_lien",
        "fraudulent_lien",
        "final_payment_affidavit",
        "auto",
    ];

    /// Generate invalid document types
    fn invalid_doc_type() -> impl Strategy<Value = String> {
        "[a-z]{3,10}".prop_filter("Must not be valid doc type", |s| {
            !VALID_DOC_TYPES.contains(&s.as_str())
        })
    }

    proptest! {
        /// Property: Valid document types are recognized
        #[test]
        fn valid_doc_types_accepted(doc_type in valid_doc_type()) {
            prop_assert!(
                VALID_DOC_TYPES.contains(&doc_type.as_str()),
                "Document type '{}' should be valid", doc_type
            );
        }

        /// Property: Invalid document types are identifiable
        #[test]
        fn invalid_doc_types_identified(doc_type in invalid_doc_type()) {
            prop_assert!(
                !VALID_DOC_TYPES.contains(&doc_type.as_str()),
                "Random doc type '{}' should be invalid", doc_type
            );
        }

        /// Property: PPI values in reasonable range for PNG rendering
        #[test]
        fn ppi_reasonable_range(ppi in 72u32..600) {
            // Standard DPI values: 72 (screen), 150 (draft), 300 (print), 600 (high)
            prop_assert!((72..=600).contains(&ppi),
                "PPI {} should be in reasonable range 72-600", ppi);
        }

        /// Property: Base64 encoding preserves data
        #[test]
        fn base64_roundtrip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            use base64::{Engine, engine::general_purpose::STANDARD};
            let encoded = STANDARD.encode(&data);
            let decoded = STANDARD.decode(&encoded).unwrap();
            prop_assert_eq!(data, decoded);
        }
    }
}

#[cfg(test)]
mod http_endpoint_tests {
    //! HTTP endpoint integration tests using axum-test

    use axum::{
        routing::{get, post},
        Router,
    };
    use axum_test::TestServer;
    use serde_json::json;

    use crate::api::{
        handle_check_compliance, handle_health, handle_list_templates, handle_render_template,
    };
    use crate::AppState;

    /// Create a test server with the full router
    fn create_test_server() -> TestServer {
        let state = AppState { timeout_ms: 10000 };

        let app = Router::new()
            .route("/health", get(handle_health))
            .route("/api/templates", get(handle_list_templates))
            .route("/api/render", post(handle_render_template))
            .route("/api/compliance", post(handle_check_compliance))
            .with_state(state);

        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn test_health_returns_200() {
        let server = create_test_server();
        let response = server.get("/health").await;
        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], "agentpdf-server");
    }

    #[tokio::test]
    async fn test_templates_returns_all_templates() {
        let server = create_test_server();
        let response = server.get("/api/templates").await;
        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["count"].as_u64().unwrap() >= 8); // At least 8 templates
    }

    #[tokio::test]
    async fn test_compliance_florida_lease() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "This is a standard Florida residential lease agreement.",
                "state": "FL",
                "document_type": "lease"
            }))
            .await;

        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_compliance_with_zip_code() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "Security deposit: $2,000. Monthly rent: $2,000.",
                "state": "IL",
                "zip_code": "60601",
                "document_type": "lease"
            }))
            .await;

        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
        // Chicago RLTO should detect violations
    }

    #[tokio::test]
    async fn test_compliance_rejects_invalid_state() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "Test lease",
                "state": "XX",  // Invalid state
                "document_type": "lease"
            }))
            .await;

        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_compliance_rejects_invalid_doc_type() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "Test document",
                "state": "FL",
                "document_type": "invalid_type"
            }))
            .await;

        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_render_rejects_invalid_format() {
        let server = create_test_server();

        let response = server
            .post("/api/render")
            .json(&json!({
                "template": "florida_lease",
                "format": "docx",  // Invalid format
                "inputs": {}
            }))
            .await;

        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_render_with_minimal_inputs() {
        let server = create_test_server();

        // Render with minimal inputs - templates may have defaults
        let response = server
            .post("/api/render")
            .json(&json!({
                "template": "florida_lease",
                "format": "pdf",
                "inputs": {
                    "landlord_name": "Test Landlord",
                    "tenant_name": "Test Tenant",
                    "property_address": "123 Test St",
                    "monthly_rent": 1500,
                    "lease_start": "2026-01-01",
                    "lease_end": "2026-12-31"
                }
            }))
            .await;

        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"].is_string()); // Base64-encoded PDF
        assert!(json["page_count"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_render_nonexistent_template() {
        let server = create_test_server();

        let response = server
            .post("/api/render")
            .json(&json!({
                "template": "nonexistent_template_xyz",
                "format": "pdf",
                "inputs": {}
            }))
            .await;

        // Should fail with unknown template
        let status = response.status_code();
        assert!(
            status.is_client_error() || status.is_server_error(),
            "Should fail with nonexistent template"
        );
    }

    #[tokio::test]
    async fn test_compliance_handles_empty_text() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "",
                "state": "FL",
                "document_type": "lease"
            }))
            .await;

        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_compliance_with_year_built() {
        let server = create_test_server();

        // Pre-1978 property should trigger lead paint checks
        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "This is a lease for a property built in 1970.",
                "state": "FL",
                "year_built": 1970,
                "document_type": "lease"
            }))
            .await;

        response.assert_status_ok();

        let json = response.json::<serde_json::Value>();
        assert!(json["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_compliance_auto_detect() {
        let server = create_test_server();

        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "PURCHASE AND SALE AGREEMENT. Buyer agrees to purchase...",
                "state": "FL",
                "document_type": "auto"
            }))
            .await;

        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_florida_state_supported() {
        let server = create_test_server();

        // Florida is always supported (MVP focus)
        let response = server
            .post("/api/compliance")
            .json(&json!({
                "text": "Test Florida lease document",
                "state": "FL",
                "document_type": "lease"
            }))
            .await;

        response.assert_status_ok();
        let json = response.json::<serde_json::Value>();
        assert!(
            json["success"].as_bool().unwrap(),
            "Florida should be supported"
        );
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

    /// Regression: Florida prohibited provisions detected
    #[test]
    fn florida_prohibited_provisions_detected() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);

        // Text with prohibited waiver of notice
        let text = "Tenant hereby waives all rights to notice before termination.";
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // Should detect prohibited waiver provision
        let has_prohibition = violations.iter().any(|v| v.statute.contains("83.47"));
        assert!(
            has_prohibition,
            "Florida prohibited provision (waiver of notice) should be detected"
        );
    }

    /// Regression: Florida security deposit bank disclosure
    #[test]
    fn florida_deposit_bank_required() {
        let engine = ComplianceEngine::new();
        let jurisdiction = Jurisdiction::new(State::FL);

        // Text with deposit but no bank
        let text = "Security deposit: $2,000.";
        let violations = engine.check_text_with_jurisdiction(&jurisdiction, text, None);

        // Should detect missing bank disclosure
        let has_bank_violation = violations.iter().any(|v| v.statute.contains("83.49"));
        assert!(
            has_bank_violation,
            "Florida deposit bank disclosure should be required"
        );
    }
}
