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

// ============================================================================
// Build Configuration Parity
// ============================================================================

/// ISSUE-016: Test that pdfjoin-web Trunk.toml has the required pre_build hook.
/// This hook compiles TypeScript (src/ts/*.ts -> www/js/bundle.js) before Trunk serves.
/// Without this, the browser tests will fail because bundle.js may be stale.
#[test]
fn test_pdfjoin_trunk_toml_has_prebuild_hook() {
    let workspace_root = paths::workspace_root();
    let trunk_toml_path = workspace_root.join("apps/pdfjoin-web/Trunk.toml");

    let content =
        std::fs::read_to_string(&trunk_toml_path).expect("Should read pdfjoin-web Trunk.toml");

    // Check that there's a [[hooks]] section with pre_build stage
    assert!(
        content.contains("[[hooks]]"),
        "pdfjoin-web Trunk.toml must have a [[hooks]] section for TypeScript compilation"
    );

    // Check for pre_build stage
    assert!(
        content.contains("stage = \"pre_build\""),
        "pdfjoin-web Trunk.toml must have a pre_build hook stage"
    );

    // Check that npm run build is called
    assert!(
        content.contains("command = \"npm\"") && content.contains("[\"run\", \"build\"]"),
        "pdfjoin-web Trunk.toml pre_build hook must run 'npm run build' to compile TypeScript"
    );

    eprintln!("✓ pdfjoin-web Trunk.toml has required pre_build hook for TypeScript");
}

/// Test that pdfjoin-web Trunk.toml watches the TypeScript source directory.
/// Without this, TypeScript changes won't trigger rebuilds during development.
#[test]
fn test_pdfjoin_trunk_toml_watches_typescript() {
    let workspace_root = paths::workspace_root();
    let trunk_toml_path = workspace_root.join("apps/pdfjoin-web/Trunk.toml");

    let content =
        std::fs::read_to_string(&trunk_toml_path).expect("Should read pdfjoin-web Trunk.toml");

    // Check that src/ts is in the watch paths
    assert!(
        content.contains("src/ts"),
        "pdfjoin-web Trunk.toml [watch] must include 'src/ts' to detect TypeScript changes"
    );

    eprintln!("✓ pdfjoin-web Trunk.toml watches src/ts for TypeScript changes");
}

/// Test that pdfjoin-web Trunk.toml ignores the bundle output to prevent rebuild loops.
/// The pre_build hook writes bundle.js, which would trigger another rebuild if not ignored.
#[test]
fn test_pdfjoin_trunk_toml_ignores_bundle_output() {
    let workspace_root = paths::workspace_root();
    let trunk_toml_path = workspace_root.join("apps/pdfjoin-web/Trunk.toml");

    let content =
        std::fs::read_to_string(&trunk_toml_path).expect("Should read pdfjoin-web Trunk.toml");

    // Check that bundle.js is ignored to prevent rebuild loops
    assert!(
        content.contains("www/js/bundle.js"),
        "pdfjoin-web Trunk.toml [watch].ignore must include 'www/js/bundle.js' to prevent rebuild loops"
    );

    eprintln!("✓ pdfjoin-web Trunk.toml ignores bundle.js to prevent rebuild loops");
}

// ============================================================================
// Change Detection Script Tests
// ============================================================================

/// Helper to run detect-changed-apps.sh with given input files
fn run_detection_script(files: &[&str]) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let workspace_root = paths::workspace_root();
    let script_path = workspace_root.join("scripts/detect-changed-apps.sh");

    let mut child = Command::new(&script_path)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run detect-changed-apps.sh");

    // Write file paths to stdin
    let input = files.join("\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("Failed to write to stdin");

    let output = child.wait_with_output().expect("Failed to wait for script");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_detect_changed_apps_pdfjoin_only() {
    let result = run_detection_script(&["apps/pdfjoin-web/src/ts/app.ts"]);
    assert_eq!(
        result, "pdfjoin",
        "pdfjoin-web changes should detect pdfjoin"
    );
}

#[test]
fn test_detect_changed_apps_agentpdf_only() {
    let result = run_detection_script(&["apps/agentpdf-web/www/index.html"]);
    assert_eq!(
        result, "agentpdf",
        "agentpdf-web changes should detect agentpdf"
    );
}

#[test]
fn test_detect_changed_apps_docsign_only() {
    let result = run_detection_script(&["apps/docsign-web/wasm/src/lib.rs"]);
    assert_eq!(
        result, "docsign",
        "docsign-web changes should detect docsign"
    );
}

#[test]
fn test_detect_changed_apps_multiple_apps() {
    let result = run_detection_script(&["apps/pdfjoin-web/foo.ts", "apps/docsign-web/bar.rs"]);
    // Result should contain both apps (order may vary)
    assert!(
        result.contains("pdfjoin") && result.contains("docsign"),
        "Multiple app changes should detect all affected apps: got '{}'",
        result
    );
}

#[test]
fn test_detect_changed_apps_crate_maps_to_app() {
    let result = run_detection_script(&["crates/pdfjoin-core/src/lib.rs"]);
    assert_eq!(
        result, "pdfjoin",
        "pdfjoin crate changes should detect pdfjoin"
    );

    let result = run_detection_script(&["crates/docsign-core/src/lib.rs"]);
    assert_eq!(
        result, "docsign",
        "docsign crate changes should detect docsign"
    );
}

#[test]
fn test_detect_changed_apps_shared_code_returns_all() {
    let result = run_detection_script(&["crates/shared-pdf/src/lib.rs"]);
    assert_eq!(result, "all", "shared-pdf changes should trigger all apps");

    let result = run_detection_script(&["crates/shared-types/src/lib.rs"]);
    assert_eq!(
        result, "all",
        "shared-types changes should trigger all apps"
    );

    let result = run_detection_script(&["crates/compliance-engine/src/lib.rs"]);
    assert_eq!(
        result, "all",
        "compliance-engine changes should trigger all apps"
    );
}

#[test]
fn test_detect_changed_apps_scripts_returns_all() {
    let result = run_detection_script(&["scripts/test-browser.sh"]);
    assert_eq!(result, "all", "scripts changes should trigger all apps");
}

#[test]
fn test_detect_changed_apps_benchmark_harness_returns_all() {
    let result = run_detection_script(&["crates/benchmark-harness/tests/browser_pdfjoin.rs"]);
    assert_eq!(
        result, "all",
        "benchmark-harness changes should trigger all apps"
    );
}

#[test]
fn test_detect_changed_apps_docs_only_returns_none() {
    let result = run_detection_script(&["README.md"]);
    assert_eq!(
        result, "none",
        "README.md only should not trigger browser tests"
    );

    let result = run_detection_script(&["docs/architecture.md"]);
    assert_eq!(
        result, "none",
        "docs changes should not trigger browser tests"
    );
}

#[test]
fn test_detect_changed_apps_mixed_docs_and_code() {
    let result = run_detection_script(&["README.md", "apps/agentpdf-web/index.html"]);
    assert_eq!(
        result, "agentpdf",
        "Mixed docs+code should detect affected app"
    );
}

#[test]
fn test_detect_changed_apps_cargo_toml_returns_all() {
    let result = run_detection_script(&["Cargo.toml"]);
    assert_eq!(result, "all", "Cargo.toml changes should trigger all apps");

    let result = run_detection_script(&["Cargo.lock"]);
    assert_eq!(result, "all", "Cargo.lock changes should trigger all apps");
}

#[test]
fn test_detect_changed_apps_empty_input_returns_none() {
    let result = run_detection_script(&[]);
    assert_eq!(result, "none", "Empty input should return none");
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
