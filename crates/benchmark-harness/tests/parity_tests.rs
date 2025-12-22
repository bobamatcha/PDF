//! Configuration and code parity tests
//!
//! These tests ensure all apps have consistent configurations and shared code.
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

/// Test that all apps have the same wasm-opt configuration in their HTML.
/// This catches the Rust 1.82+ bulk-memory regression where one app might
/// have the fix but not the other.
#[test]
fn test_wasm_opt_config_parity() {
    let workspace_root = paths::workspace_root();

    let apps = [
        (
            "agentpdf-web",
            workspace_root.join("apps/agentpdf-web/www/index.html"),
        ),
        (
            "docsign-web",
            workspace_root.join("apps/docsign-web/www/index.html"),
        ),
        (
            "pdfjoin-web",
            workspace_root.join("apps/pdfjoin-web/www/index.html"),
        ),
    ];

    let extract_wasm_opt_params = |content: &str| -> Option<String> {
        let pattern = r#"data-wasm-opt-params="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    let mut all_params: Vec<(String, Option<String>)> = Vec::new();

    for (name, path) in &apps {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Should read {} index.html at {:?}", name, path));
        let params = extract_wasm_opt_params(&content);
        all_params.push((name.to_string(), params));
    }

    // Verify all apps have wasm-opt-params configured
    for (name, params) in &all_params {
        assert!(
            params.is_some(),
            "{} should have data-wasm-opt-params configured for Rust 1.82+ compatibility",
            name
        );
    }

    // Verify all apps have identical params
    let first_params = all_params[0].1.as_ref().unwrap();
    for (name, params) in &all_params[1..] {
        assert_eq!(
            params.as_ref().unwrap(),
            first_params,
            "All apps should have identical wasm-opt-params. {} differs from {}",
            name,
            all_params[0].0
        );
    }

    // Verify required flags
    assert!(
        first_params.contains("--enable-bulk-memory"),
        "wasm-opt-params must include --enable-bulk-memory for Rust 1.82+"
    );
    assert!(
        first_params.contains("--enable-nontrapping-float-to-int"),
        "wasm-opt-params should include --enable-nontrapping-float-to-int"
    );

    eprintln!(
        "✓ All {} apps have matching wasm-opt config: {}",
        apps.len(),
        first_params
    );
}

/// Test that all apps have the same wasm-opt optimization level
#[test]
fn test_wasm_opt_level_parity() {
    let workspace_root = paths::workspace_root();

    let apps = [
        (
            "agentpdf-web",
            workspace_root.join("apps/agentpdf-web/www/index.html"),
        ),
        (
            "docsign-web",
            workspace_root.join("apps/docsign-web/www/index.html"),
        ),
        (
            "pdfjoin-web",
            workspace_root.join("apps/pdfjoin-web/www/index.html"),
        ),
    ];

    let extract_wasm_opt_level = |content: &str| -> Option<String> {
        let pattern = r#"data-wasm-opt="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    let mut all_levels: Vec<(String, Option<String>)> = Vec::new();

    for (name, path) in &apps {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Should read {} index.html at {:?}", name, path));
        let level = extract_wasm_opt_level(&content);
        all_levels.push((name.to_string(), level));
    }

    // Verify all apps have the same level
    let first_level = &all_levels[0].1;
    for (name, level) in &all_levels[1..] {
        assert_eq!(
            level, first_level,
            "All apps should have same wasm-opt level. {} differs from {}",
            name, all_levels[0].0
        );
    }

    eprintln!(
        "✓ All {} apps have matching wasm-opt level: {:?}",
        apps.len(),
        first_level
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

/// Test that all apps have proper viewport meta tag for mobile support
#[test]
fn test_viewport_meta_parity() {
    let workspace_root = paths::workspace_root();

    let apps = [
        (
            "agentpdf-web",
            workspace_root.join("apps/agentpdf-web/www/index.html"),
        ),
        (
            "docsign-web",
            workspace_root.join("apps/docsign-web/www/index.html"),
        ),
        (
            "pdfjoin-web",
            workspace_root.join("apps/pdfjoin-web/www/index.html"),
        ),
    ];

    let extract_viewport = |content: &str| -> Option<String> {
        let pattern = r#"<meta\s+name="viewport"\s+content="([^"]*)""#;
        let re = Regex::new(pattern).unwrap();
        re.captures(content)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    };

    eprintln!("Viewport meta tags:");
    for (name, path) in &apps {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Should read {} index.html at {:?}", name, path));
        let viewport = extract_viewport(&content);

        assert!(viewport.is_some(), "{} should have viewport meta tag", name);

        let vp = viewport.unwrap();
        assert!(
            vp.contains("width=device-width"),
            "{} viewport should include width=device-width",
            name
        );

        eprintln!("  {}: {}", name, vp);
    }

    eprintln!("✓ All {} apps have proper viewport meta tags", apps.len());
}
