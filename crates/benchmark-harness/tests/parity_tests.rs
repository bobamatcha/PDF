//! Configuration and code parity tests
//!
//! These tests ensure both apps have consistent configurations and shared code.
//! They don't require a browser or server - they're pure file-based checks.
//!
//! Run with: cargo test -p benchmark-harness --test parity_tests

#[path = "common/paths.rs"]
mod paths;

use regex::Regex;
use std::collections::HashSet;

// ============================================================================
// WASM Build Configuration Parity
// ============================================================================

/// Test that both apps have the same wasm-opt configuration in their HTML.
/// This catches the Rust 1.82+ bulk-memory regression where one app might
/// have the fix but not the other.
#[test]
fn test_wasm_opt_config_parity() {
    let workspace_root = paths::workspace_root();

    let agentpdf_html = workspace_root.join("apps/agentpdf-web/www/index.html");
    let docsign_html = workspace_root.join("apps/docsign-web/www/index.html");

    let agentpdf_content = std::fs::read_to_string(&agentpdf_html)
        .unwrap_or_else(|_| panic!("Should read agentpdf index.html at {:?}", agentpdf_html));
    let docsign_content = std::fs::read_to_string(&docsign_html)
        .unwrap_or_else(|_| panic!("Should read docsign index.html at {:?}", docsign_html));

    let extract_wasm_opt_params = |content: &str| -> Option<String> {
        let pattern = r#"data-wasm-opt-params="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    let agentpdf_params = extract_wasm_opt_params(&agentpdf_content);
    let docsign_params = extract_wasm_opt_params(&docsign_content);

    assert!(
        agentpdf_params.is_some(),
        "agentpdf-web should have data-wasm-opt-params configured for Rust 1.82+ compatibility"
    );
    assert!(
        docsign_params.is_some(),
        "docsign-web should have data-wasm-opt-params configured for Rust 1.82+ compatibility"
    );

    assert_eq!(
        agentpdf_params, docsign_params,
        "Both apps should have identical wasm-opt-params. agentpdf: {:?}, docsign: {:?}",
        agentpdf_params, docsign_params
    );

    let params = agentpdf_params.unwrap();
    assert!(
        params.contains("--enable-bulk-memory"),
        "wasm-opt-params must include --enable-bulk-memory for Rust 1.82+"
    );
    assert!(
        params.contains("--enable-nontrapping-float-to-int"),
        "wasm-opt-params should include --enable-nontrapping-float-to-int"
    );

    eprintln!("✓ Both apps have matching wasm-opt config: {}", params);
}

/// Test that both apps have the same wasm-opt optimization level
#[test]
fn test_wasm_opt_level_parity() {
    let workspace_root = paths::workspace_root();

    let agentpdf_html = workspace_root.join("apps/agentpdf-web/www/index.html");
    let docsign_html = workspace_root.join("apps/docsign-web/www/index.html");

    let agentpdf_content =
        std::fs::read_to_string(&agentpdf_html).expect("Should read agentpdf index.html");
    let docsign_content =
        std::fs::read_to_string(&docsign_html).expect("Should read docsign index.html");

    let extract_wasm_opt_level = |content: &str| -> Option<String> {
        let pattern = r#"data-wasm-opt="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    let agentpdf_level = extract_wasm_opt_level(&agentpdf_content);
    let docsign_level = extract_wasm_opt_level(&docsign_content);

    assert_eq!(
        agentpdf_level, docsign_level,
        "Both apps should have same wasm-opt level. agentpdf: {:?}, docsign: {:?}",
        agentpdf_level, docsign_level
    );

    eprintln!(
        "✓ Both apps have matching wasm-opt level: {:?}",
        agentpdf_level
    );
}

// ============================================================================
// Shared Code Parity
// ============================================================================

/// Test that duplicated source files (coords.rs) have matching test coverage.
/// This catches when a bug fix or new test is added to one app but not the other.
#[test]
fn test_coords_rs_test_parity() {
    let workspace_root = paths::workspace_root();

    let agentpdf_coords = workspace_root.join("apps/agentpdf-web/wasm/src/coords.rs");
    let docsign_coords = workspace_root.join("apps/docsign-web/wasm/src/coords.rs");

    let agentpdf_content =
        std::fs::read_to_string(&agentpdf_coords).expect("Should read agentpdf coords.rs");
    let docsign_content =
        std::fs::read_to_string(&docsign_coords).expect("Should read docsign coords.rs");

    let extract_test_names = |content: &str| -> Vec<String> {
        let pattern = r#"fn (test_[a-z_]+|roundtrip_[a-z_]+|origin_[a-z_]+|linear_[a-z_]+|offset_[a-z_]+)\s*\("#;
        let re = Regex::new(pattern).unwrap();
        re.captures_iter(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect()
    };

    let agentpdf_tests: HashSet<_> = extract_test_names(&agentpdf_content).into_iter().collect();
    let docsign_tests: HashSet<_> = extract_test_names(&docsign_content).into_iter().collect();

    let only_in_agentpdf: Vec<_> = agentpdf_tests.difference(&docsign_tests).collect();
    let only_in_docsign: Vec<_> = docsign_tests.difference(&agentpdf_tests).collect();

    eprintln!("agentpdf coords.rs tests: {:?}", agentpdf_tests);
    eprintln!("docsign coords.rs tests: {:?}", docsign_tests);

    if !only_in_agentpdf.is_empty() {
        eprintln!("⚠️  Tests only in agentpdf: {:?}", only_in_agentpdf);
    }
    if !only_in_docsign.is_empty() {
        eprintln!("⚠️  Tests only in docsign: {:?}", only_in_docsign);
    }

    // Strict enforcement - fail if tests don't match
    assert!(
        only_in_agentpdf.is_empty() && only_in_docsign.is_empty(),
        "coords.rs test coverage must match between apps!\n  Only in agentpdf: {:?}\n  Only in docsign: {:?}",
        only_in_agentpdf,
        only_in_docsign
    );

    eprintln!("✓ coords.rs has matching test coverage in both apps");
}

/// Test that both apps have the same viewport meta tag for mobile support
#[test]
fn test_viewport_meta_parity() {
    let workspace_root = paths::workspace_root();

    let agentpdf_html = workspace_root.join("apps/agentpdf-web/www/index.html");
    let docsign_html = workspace_root.join("apps/docsign-web/www/index.html");

    let agentpdf_content =
        std::fs::read_to_string(&agentpdf_html).expect("Should read agentpdf index.html");
    let docsign_content =
        std::fs::read_to_string(&docsign_html).expect("Should read docsign index.html");

    let extract_viewport = |content: &str| -> Option<String> {
        let pattern = r#"<meta\s+name="viewport"\s+content="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    let agentpdf_viewport = extract_viewport(&agentpdf_content);
    let docsign_viewport = extract_viewport(&docsign_content);

    assert!(
        agentpdf_viewport.is_some(),
        "agentpdf should have viewport meta tag"
    );
    assert!(
        docsign_viewport.is_some(),
        "docsign should have viewport meta tag"
    );

    // Both should have width=device-width for proper mobile scaling
    let agentpdf_vp = agentpdf_viewport.unwrap();
    let docsign_vp = docsign_viewport.unwrap();

    assert!(
        agentpdf_vp.contains("width=device-width"),
        "agentpdf viewport should include width=device-width"
    );
    assert!(
        docsign_vp.contains("width=device-width"),
        "docsign viewport should include width=device-width"
    );

    eprintln!("✓ Both apps have proper viewport meta tags");
    eprintln!("  agentpdf: {}", agentpdf_vp);
    eprintln!("  docsign: {}", docsign_vp);
}
