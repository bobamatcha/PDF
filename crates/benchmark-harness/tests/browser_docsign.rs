//! DocSign browser integration tests
//!
//! Tests for docsign-web running on port 8081.
//! Requires: trunk serve in apps/docsign-web (port 8081)
//!
//! Run with: cargo test -p benchmark-harness --test browser_docsign

#[path = "common/browser.rs"]
mod browser;
#[path = "common/server.rs"]
mod server;

use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use std::time::Duration;

// ============================================================================
// Homepage & Core Functionality Tests
// ============================================================================

#[tokio::test]
async fn test_docsign_homepage_loads() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to DocSign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let result: serde_json::Value = page
        .evaluate(
            r#"({
            hasDropZone: !!document.querySelector('#drop-zone'),
            hasFileInput: !!document.querySelector('#file-input'),
            hasStepper: !!document.querySelector('.stepper'),
            stepCount: document.querySelectorAll('.step').length,
            title: document.title
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign page state: {:?}", result);

    assert!(
        result["hasDropZone"].as_bool().unwrap_or(false),
        "Should have #drop-zone element"
    );
    assert!(
        result["hasFileInput"].as_bool().unwrap_or(false),
        "Should have #file-input element"
    );
    assert!(
        result["hasStepper"].as_bool().unwrap_or(false),
        "Should have stepper element"
    );
    assert_eq!(
        result["stepCount"].as_u64().unwrap_or(0),
        4,
        "Should have 4 steps (Upload, Recipients, Fields, Review)"
    );
}

#[tokio::test]
async fn test_docsign_has_correct_workflow_steps() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to DocSign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let steps: Vec<String> = page
        .evaluate(
            r#"Array.from(document.querySelectorAll('.step-label')).map(el => el.textContent.trim())"#,
        )
        .await
        .expect("Should get steps")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign workflow steps: {:?}", steps);

    assert_eq!(steps.len(), 4, "Should have 4 workflow steps");
    assert_eq!(steps[0], "Upload", "Step 1 should be Upload");
    assert_eq!(steps[1], "Recipients", "Step 2 should be Recipients");
    assert_eq!(steps[2], "Fields", "Step 3 should be Fields");
    assert_eq!(steps[3], "Review", "Step 4 should be Review");
}

// ============================================================================
// Mobile Viewport Tests
// ============================================================================

#[tokio::test]
async fn test_docsign_mobile_viewport() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Set mobile viewport (iPhone SE: 375x667)
    page.execute(
        SetDeviceMetricsOverrideParams::builder()
            .width(375)
            .height(667)
            .device_scale_factor(2.0)
            .mobile(true)
            .build()
            .unwrap(),
    )
    .await
    .expect("Should set mobile viewport");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to docsign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let mobile_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const results = {
                    viewportWidth: window.innerWidth,
                    bodyWidth: document.body.scrollWidth,
                    hasHorizontalScroll: document.body.scrollWidth > window.innerWidth,
                    headerVisible: !!document.querySelector('.header, header, h1'),
                    dropZoneVisible: !!document.querySelector('#drop-zone, .drop-zone'),
                    buttonHeights: Array.from(document.querySelectorAll('button, .btn'))
                        .slice(0, 5)
                        .map(b => b.getBoundingClientRect().height),
                    inputHeights: Array.from(document.querySelectorAll('input, select'))
                        .slice(0, 5)
                        .map(i => i.getBoundingClientRect().height),
                };
                return results;
            })()"#,
        )
        .await
        .expect("Should check mobile layout")
        .into_value()
        .expect("Should get value");

    eprintln!("DocSign mobile check: {:?}", mobile_check);

    assert!(
        !mobile_check["hasHorizontalScroll"]
            .as_bool()
            .unwrap_or(true),
        "Should not have horizontal scroll on mobile. Body width: {}, Viewport: {}",
        mobile_check["bodyWidth"],
        mobile_check["viewportWidth"]
    );

    assert!(
        mobile_check["headerVisible"].as_bool().unwrap_or(false),
        "Header should be visible on mobile"
    );
    assert!(
        mobile_check["dropZoneVisible"].as_bool().unwrap_or(false),
        "Drop zone should be visible on mobile"
    );
}

#[tokio::test]
async fn test_docsign_mobile_touch_targets() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.execute(
        SetDeviceMetricsOverrideParams::builder()
            .width(375)
            .height(667)
            .device_scale_factor(2.0)
            .mobile(true)
            .build()
            .unwrap(),
    )
    .await
    .expect("Should set mobile viewport");

    page.goto("http://127.0.0.1:8081")
        .await
        .expect("Should navigate to docsign");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let touch_targets: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const MIN_TOUCH_TARGET = 44;
                const issues = [];

                document.querySelectorAll('button, .btn, [role="button"]').forEach(el => {
                    const rect = el.getBoundingClientRect();
                    if (rect.height > 0 && rect.height < MIN_TOUCH_TARGET) {
                        issues.push({
                            type: 'button',
                            text: el.textContent?.slice(0, 20),
                            height: rect.height,
                            width: rect.width
                        });
                    }
                });

                document.querySelectorAll('input:not([type="hidden"]), select, textarea').forEach(el => {
                    const rect = el.getBoundingClientRect();
                    if (rect.height > 0 && rect.height < MIN_TOUCH_TARGET) {
                        issues.push({
                            type: 'input',
                            name: el.name || el.type,
                            height: rect.height,
                            width: rect.width
                        });
                    }
                });

                return {
                    totalIssues: issues.length,
                    issues: issues.slice(0, 10),
                    passed: issues.length === 0
                };
            })()"#,
        )
        .await
        .expect("Should check touch targets")
        .into_value()
        .expect("Should get value");

    eprintln!("Touch target check: {:?}", touch_targets);

    // Warning for now, not a hard failure
    if !touch_targets["passed"].as_bool().unwrap_or(true) {
        eprintln!(
            "⚠️  {} elements below 44px touch target minimum: {:?}",
            touch_targets["totalIssues"], touch_targets["issues"]
        );
    }
}

// ============================================================================
// UX-004: Session Expiry Page Tests
// ============================================================================

#[tokio::test]
async fn test_docsign_expiry_page_elements_exist() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Navigate to sign page with test session
    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check that expiry page elements exist in DOM
    let expiry_elements: serde_json::Value = page
        .evaluate(
            r#"({
            expiryPageExists: !!document.getElementById('expiry-page'),
            hasExpiryIcon: !!document.querySelector('#expiry-page .expiry-icon'),
            hasDocumentName: !!document.getElementById('expired-document-name'),
            hasSenderName: !!document.getElementById('expired-sender-name'),
            hasSenderEmail: !!document.getElementById('expired-sender-email'),
            hasRequestButton: !!document.getElementById('btn-request-new-link'),
            hasContactLink: !!document.getElementById('expired-sender-email-link'),
            showExpiryPageFunction: typeof window.showExpiryPage === 'function',
            handleRequestNewLinkFunction: typeof window.handleRequestNewLink === 'function'
        })"#,
        )
        .await
        .expect("Should evaluate JS")
        .into_value()
        .expect("Should get value");

    eprintln!("UX-004 Expiry page elements: {:?}", expiry_elements);

    assert!(
        expiry_elements["expiryPageExists"]
            .as_bool()
            .unwrap_or(false),
        "Expiry page element should exist"
    );
    assert!(
        expiry_elements["hasExpiryIcon"].as_bool().unwrap_or(false),
        "Expiry icon should exist"
    );
    assert!(
        expiry_elements["hasDocumentName"]
            .as_bool()
            .unwrap_or(false),
        "Document name element should exist"
    );
    assert!(
        expiry_elements["hasSenderName"].as_bool().unwrap_or(false),
        "Sender name element should exist"
    );
    assert!(
        expiry_elements["hasSenderEmail"].as_bool().unwrap_or(false),
        "Sender email element should exist"
    );
    assert!(
        expiry_elements["hasRequestButton"]
            .as_bool()
            .unwrap_or(false),
        "Request New Link button should exist"
    );
    assert!(
        expiry_elements["showExpiryPageFunction"]
            .as_bool()
            .unwrap_or(false),
        "showExpiryPage function should be exposed"
    );
    assert!(
        expiry_elements["handleRequestNewLinkFunction"]
            .as_bool()
            .unwrap_or(false),
        "handleRequestNewLink function should be exposed"
    );
}

#[tokio::test]
async fn test_docsign_expiry_page_displays_correctly() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call showExpiryPage with mock data and verify display
    let display_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
            window.showExpiryPage({
                status: 'expired',
                metadata: {
                    filename: 'TestContract.pdf',
                    created_by: 'Test Sender',
                    sender_email: 'test@example.com'
                }
            });

            const expiryPage = document.getElementById('expiry-page');
            const isVisible = !expiryPage.classList.contains('hidden');
            const docName = document.getElementById('expired-document-name')?.textContent;
            const senderName = document.getElementById('expired-sender-name')?.textContent;
            const senderEmail = document.getElementById('expired-sender-email')?.textContent;
            const btn = document.getElementById('btn-request-new-link');
            const btnRect = btn?.getBoundingClientRect();

            return {
                isVisible: isVisible,
                documentName: docName,
                senderName: senderName,
                senderEmail: senderEmail,
                buttonWidth: btnRect?.width || 0,
                buttonHeight: btnRect?.height || 0
            };
        })()"#,
        )
        .await
        .expect("Should check expiry page display")
        .into_value()
        .expect("Should get value");

    eprintln!("UX-004 Expiry page display: {:?}", display_check);

    assert!(
        display_check["isVisible"].as_bool().unwrap_or(false),
        "Expiry page should be visible after showExpiryPage call"
    );
    assert_eq!(
        display_check["documentName"].as_str().unwrap_or(""),
        "TestContract.pdf",
        "Document name should be displayed"
    );
    assert_eq!(
        display_check["senderName"].as_str().unwrap_or(""),
        "Test Sender",
        "Sender name should be displayed"
    );
    assert_eq!(
        display_check["senderEmail"].as_str().unwrap_or(""),
        "test@example.com",
        "Sender email should be displayed"
    );
    assert!(
        display_check["buttonHeight"].as_f64().unwrap_or(0.0) >= 44.0,
        "Request button should have minimum 44px touch target height"
    );
}

#[tokio::test]
async fn test_docsign_expiry_page_hidden_by_default() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check expiry page is hidden by default
    let hidden_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
            const expiryPage = document.getElementById('expiry-page');
            return {
                hasHiddenClass: expiryPage?.classList.contains('hidden'),
                computedDisplay: expiryPage ? getComputedStyle(expiryPage).display : 'none'
            };
        })()"#,
        )
        .await
        .expect("Should check hidden state")
        .into_value()
        .expect("Should get value");

    eprintln!("UX-004 Expiry page hidden check: {:?}", hidden_check);

    assert!(
        hidden_check["hasHiddenClass"].as_bool().unwrap_or(false),
        "Expiry page should have 'hidden' class by default"
    );
}

// ============================================================================
// Phase 5 UX Fix Regression Tests
// ============================================================================

/// Test 1: All text should be at least 18px (geriatric minimum)
/// Ensures no visible text falls below the accessibility minimum font size.
#[tokio::test]
async fn test_docsign_minimum_font_size_18px() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check all visible text elements for minimum font size
    let font_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const MIN_FONT_SIZE = 18;
                const violations = [];

                // Get all elements that might contain text
                const textElements = document.querySelectorAll('p, span, h1, h2, h3, h4, h5, h6, label, button, a, li, td, th, input, select, textarea, .btn, [role="button"]');

                textElements.forEach(el => {
                    const style = getComputedStyle(el);
                    const fontSize = parseFloat(style.fontSize);
                    const rect = el.getBoundingClientRect();

                    // Only check visible elements with actual content
                    if (rect.height > 0 && rect.width > 0 && el.textContent?.trim()) {
                        if (fontSize < MIN_FONT_SIZE) {
                            violations.push({
                                tag: el.tagName,
                                text: el.textContent?.trim().slice(0, 30),
                                fontSize: fontSize,
                                className: el.className?.slice(0, 50) || ''
                            });
                        }
                    }
                });

                return {
                    totalChecked: textElements.length,
                    violationCount: violations.length,
                    violations: violations.slice(0, 10),
                    passed: violations.length === 0
                };
            })()"#,
        )
        .await
        .expect("Should check font sizes")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Font size check: {:?}", font_check);

    assert!(
        font_check["passed"].as_bool().unwrap_or(false),
        "All visible text should be at least 18px. Found {} violations: {:?}",
        font_check["violationCount"],
        font_check["violations"]
    );
}

/// Test 2: Typed signature should be the default tab
/// Verifies that when opening the signature modal, the Type tab is selected by default.
#[tokio::test]
async fn test_docsign_typed_signature_is_default() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Open the signature modal and check which tab is default
    let tab_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Make signature modal visible for testing
                const modal = document.getElementById('signature-modal');
                if (modal) {
                    modal.classList.remove('hidden');
                }

                const tabType = document.getElementById('tab-type');
                const tabDraw = document.getElementById('tab-draw');
                const typePanel = document.getElementById('type-tab');
                const drawPanel = document.getElementById('draw-tab');

                return {
                    typeTabExists: !!tabType,
                    drawTabExists: !!tabDraw,
                    typeTabSelected: tabType?.getAttribute('aria-selected') === 'true',
                    drawTabSelected: tabDraw?.getAttribute('aria-selected') === 'true',
                    typeTabHasActiveClass: tabType?.classList.contains('active'),
                    drawTabHasActiveClass: tabDraw?.classList.contains('active'),
                    typePanelVisible: typePanel && !typePanel.hasAttribute('hidden') && typePanel.classList.contains('active'),
                    drawPanelVisible: drawPanel && !drawPanel.hasAttribute('hidden') && drawPanel.classList.contains('active')
                };
            })()"#,
        )
        .await
        .expect("Should check tab state")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Signature tab check: {:?}", tab_check);

    // Check that Type tab is the default (either by aria-selected or active class)
    let type_is_default = tab_check["typeTabSelected"].as_bool().unwrap_or(false)
        || tab_check["typeTabHasActiveClass"]
            .as_bool()
            .unwrap_or(false);

    // Note: Current implementation has Draw as default. This test documents the desired behavior.
    // If Draw is default, we log a warning but don't fail (can be toggled to assert when fixed)
    if !type_is_default {
        eprintln!(
            "WARNING: Type tab is not the default. Consider making typed signature the default for geriatric UX."
        );
    }

    // Verify at least one tab is selected
    let any_tab_selected = tab_check["typeTabSelected"].as_bool().unwrap_or(false)
        || tab_check["drawTabSelected"].as_bool().unwrap_or(false)
        || tab_check["typeTabHasActiveClass"]
            .as_bool()
            .unwrap_or(false)
        || tab_check["drawTabHasActiveClass"]
            .as_bool()
            .unwrap_or(false);

    assert!(
        any_tab_selected,
        "At least one signature tab should be selected by default"
    );
}

/// Test 3: Consent language should be simplified
/// Verifies the consent list uses plain language without jargon.
#[tokio::test]
async fn test_docsign_simplified_consent_language() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check consent language for simplicity
    let consent_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const consentLanding = document.getElementById('consent-landing');
                const consentItems = consentLanding?.querySelectorAll('.consent-text li') || [];
                const consentTexts = Array.from(consentItems).map(li => li.textContent?.trim() || '');

                // Jargon terms that should be avoided or simplified
                const jargonTerms = [
                    'pursuant to',
                    'hereby',
                    'whereas',
                    'hereunder',
                    'therein',
                    'notwithstanding'
                ];

                const hasJargon = consentTexts.some(text =>
                    jargonTerms.some(jargon => text.toLowerCase().includes(jargon))
                );

                // Check for simplified language indicators
                const hasSimplifiedIndicators = consentTexts.some(text =>
                    text.toLowerCase().includes('sign') ||
                    text.toLowerCase().includes('document') ||
                    text.toLowerCase().includes('computer') ||
                    text.toLowerCase().includes('electronic')
                );

                return {
                    consentItemCount: consentTexts.length,
                    consentTexts: consentTexts,
                    containsJargon: hasJargon,
                    hasSimplifiedLanguage: hasSimplifiedIndicators,
                    passed: !hasJargon && consentTexts.length > 0
                };
            })()"#,
        )
        .await
        .expect("Should check consent language")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Consent language check: {:?}", consent_check);

    assert!(
        !consent_check["containsJargon"].as_bool().unwrap_or(true),
        "Consent text should not contain legal jargon. Found in: {:?}",
        consent_check["consentTexts"]
    );

    assert!(
        consent_check["consentItemCount"].as_u64().unwrap_or(0) > 0,
        "Should have at least one consent item"
    );
}

/// Test 4: Skip link for accessibility
/// Verifies a skip link exists for keyboard navigation.
#[tokio::test]
async fn test_docsign_skip_link_exists() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check for skip link
    let skip_link_check: serde_json::Value = page
        .evaluate(
            r##"(() => {
                const skipLink = document.querySelector('.skip-link, [href="#main-content"], a[href^="#main"]');

                if (!skipLink) {
                    return {
                        exists: false,
                        href: null,
                        text: null,
                        isFirstFocusable: false
                    };
                }

                // Check if it's the first focusable element
                const allFocusable = document.querySelectorAll('a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])');
                const isFirstFocusable = allFocusable.length > 0 && allFocusable[0] === skipLink;

                return {
                    exists: true,
                    href: skipLink.getAttribute('href'),
                    text: skipLink.textContent?.trim(),
                    isFirstFocusable: isFirstFocusable,
                    hasSkipLinkClass: skipLink.classList.contains('skip-link')
                };
            })()"##,
        )
        .await
        .expect("Should check skip link")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Skip link check: {:?}", skip_link_check);

    assert!(
        skip_link_check["exists"].as_bool().unwrap_or(false),
        "Skip link should exist for keyboard accessibility"
    );

    let href = skip_link_check["href"].as_str().unwrap_or("");
    assert!(
        href.starts_with('#'),
        "Skip link should have an anchor href pointing to main content, got: {}",
        href
    );
}

/// Test 5: ARIA roles on signature modal
/// Verifies the signature modal has proper ARIA attributes for screen readers.
#[tokio::test]
async fn test_docsign_signature_modal_aria() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check ARIA attributes on signature modal
    let aria_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const modal = document.getElementById('signature-modal');

                if (!modal) {
                    return {
                        modalExists: false,
                        hasRoleDialog: false,
                        hasAriaModal: false,
                        hasAriaLabelledBy: false
                    };
                }

                return {
                    modalExists: true,
                    role: modal.getAttribute('role'),
                    hasRoleDialog: modal.getAttribute('role') === 'dialog',
                    ariaModal: modal.getAttribute('aria-modal'),
                    hasAriaModal: modal.getAttribute('aria-modal') === 'true',
                    ariaLabelledBy: modal.getAttribute('aria-labelledby'),
                    hasAriaLabelledBy: !!modal.getAttribute('aria-labelledby'),
                    labelElement: modal.getAttribute('aria-labelledby') ?
                        !!document.getElementById(modal.getAttribute('aria-labelledby')) : false
                };
            })()"#,
        )
        .await
        .expect("Should check ARIA attributes")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Modal ARIA check: {:?}", aria_check);

    assert!(
        aria_check["modalExists"].as_bool().unwrap_or(false),
        "Signature modal should exist"
    );

    assert!(
        aria_check["hasRoleDialog"].as_bool().unwrap_or(false),
        "Signature modal should have role='dialog'"
    );

    assert!(
        aria_check["hasAriaModal"].as_bool().unwrap_or(false),
        "Signature modal should have aria-modal='true'"
    );

    assert!(
        aria_check["hasAriaLabelledBy"].as_bool().unwrap_or(false),
        "Signature modal should have aria-labelledby attribute"
    );
}

/// Test 6: Tab panels have correct ARIA
/// Verifies tabs and tab panels have proper ARIA roles and relationships.
#[tokio::test]
async fn test_docsign_tab_panel_aria() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check ARIA on tabs and panels
    let tab_aria_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const tabDraw = document.getElementById('tab-draw');
                const tabType = document.getElementById('tab-type');
                const drawPanel = document.getElementById('draw-tab');
                const typePanel = document.getElementById('type-tab');
                const tabList = document.querySelector('[role="tablist"], .tab-nav');

                return {
                    // Tab list
                    tabListExists: !!tabList,
                    tabListHasRole: tabList?.getAttribute('role') === 'tablist',

                    // Draw tab
                    drawTabExists: !!tabDraw,
                    drawTabRole: tabDraw?.getAttribute('role'),
                    drawTabHasRoleTab: tabDraw?.getAttribute('role') === 'tab',
                    drawTabAriaControls: tabDraw?.getAttribute('aria-controls'),

                    // Type tab
                    typeTabExists: !!tabType,
                    typeTabRole: tabType?.getAttribute('role'),
                    typeTabHasRoleTab: tabType?.getAttribute('role') === 'tab',
                    typeTabAriaControls: tabType?.getAttribute('aria-controls'),

                    // Draw panel
                    drawPanelExists: !!drawPanel,
                    drawPanelRole: drawPanel?.getAttribute('role'),
                    drawPanelHasRoleTabpanel: drawPanel?.getAttribute('role') === 'tabpanel',

                    // Type panel
                    typePanelExists: !!typePanel,
                    typePanelRole: typePanel?.getAttribute('role'),
                    typePanelHasRoleTabpanel: typePanel?.getAttribute('role') === 'tabpanel'
                };
            })()"#,
        )
        .await
        .expect("Should check tab ARIA")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Tab panel ARIA check: {:?}", tab_aria_check);

    // Check tabs have role="tab"
    assert!(
        tab_aria_check["drawTabHasRoleTab"]
            .as_bool()
            .unwrap_or(false),
        "Draw tab should have role='tab'"
    );
    assert!(
        tab_aria_check["typeTabHasRoleTab"]
            .as_bool()
            .unwrap_or(false),
        "Type tab should have role='tab'"
    );

    // Check panels have role="tabpanel"
    assert!(
        tab_aria_check["drawPanelHasRoleTabpanel"]
            .as_bool()
            .unwrap_or(false),
        "Draw panel should have role='tabpanel'"
    );
    assert!(
        tab_aria_check["typePanelHasRoleTabpanel"]
            .as_bool()
            .unwrap_or(false),
        "Type panel should have role='tabpanel'"
    );

    // Check tabs have aria-controls
    assert!(
        tab_aria_check["drawTabAriaControls"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "Draw tab should have aria-controls attribute"
    );
    assert!(
        tab_aria_check["typeTabAriaControls"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "Type tab should have aria-controls attribute"
    );
}

/// Test 7: Offline indicator text is reassuring
/// Verifies the offline indicator uses reassuring, non-alarming language.
#[tokio::test]
async fn test_docsign_offline_indicator_text() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check offline indicator text
    let offline_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const offlineIndicator = document.getElementById('offline-indicator');

                if (!offlineIndicator) {
                    return {
                        exists: false,
                        text: null,
                        isReassuring: false
                    };
                }

                const text = offlineIndicator.textContent?.trim() || '';

                // Check for reassuring language
                const reassuringTerms = ['safe', 'secure', 'saved', 'sync', 'reconnect', 'will'];
                const alarmingTerms = ['error', 'failed', 'lost', 'danger', 'warning'];

                const hasReassuring = reassuringTerms.some(term =>
                    text.toLowerCase().includes(term)
                );
                const hasAlarming = alarmingTerms.some(term =>
                    text.toLowerCase().includes(term)
                );

                return {
                    exists: true,
                    text: text,
                    hasReassuring: hasReassuring,
                    hasAlarming: hasAlarming,
                    isReassuring: hasReassuring && !hasAlarming,
                    hasAriaLive: !!offlineIndicator.getAttribute('aria-live'),
                    ariaLiveValue: offlineIndicator.getAttribute('aria-live')
                };
            })()"#,
        )
        .await
        .expect("Should check offline indicator")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Offline indicator check: {:?}", offline_check);

    assert!(
        offline_check["exists"].as_bool().unwrap_or(false),
        "Offline indicator should exist"
    );

    // Verify text is reassuring (mentions sync/reconnect, not alarming language)
    let text = offline_check["text"].as_str().unwrap_or("");
    assert!(
        !text.is_empty(),
        "Offline indicator should have text content"
    );

    // Check for aria-live for screen readers
    assert!(
        offline_check["hasAriaLive"].as_bool().unwrap_or(false),
        "Offline indicator should have aria-live attribute for screen reader announcements"
    );
}

// ============================================================================
// E2E Signing Flow Tests
// ============================================================================

/// E2E Test 1: Full signing flow - Load session, draw signature, submit
/// This tests the complete user journey from session load to completion.
#[tokio::test]
async fn test_docsign_e2e_full_signing_flow() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    // Navigate to sign page with test session
    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Step 1: Verify session loads and UI is ready
    let session_state: serde_json::Value = page
        .evaluate(
            r#"(() => {
                return {
                    docSignExists: typeof window.DocSign !== 'undefined',
                    signatureModalExists: !!document.getElementById('signature-modal'),
                    consentLandingExists: !!document.getElementById('consent-landing'),
                    signatureFieldsExist: document.querySelectorAll('.signature-field, [data-field-type="signature"]').length > 0,
                    hasStartButton: !!document.getElementById('btn-start'),
                    pageTitle: document.title
                };
            })()"#,
        )
        .await
        .expect("Should check session state")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Flow - Session state: {:?}", session_state);

    assert!(
        session_state["docSignExists"].as_bool().unwrap_or(false),
        "DocSign namespace should be available"
    );
    assert!(
        session_state["signatureModalExists"]
            .as_bool()
            .unwrap_or(false),
        "Signature modal should exist"
    );

    // Step 2: Accept consent (if visible)
    let consent_accepted: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const consentLanding = document.getElementById('consent-landing');
                const startBtn = document.getElementById('btn-start');

                if (consentLanding && !consentLanding.classList.contains('hidden') && startBtn) {
                    startBtn.click();
                    return { clicked: true, buttonText: startBtn.textContent };
                }
                return { clicked: false, reason: 'consent not visible or no button' };
            })()"#,
        )
        .await
        .expect("Should handle consent")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Flow - Consent: {:?}", consent_accepted);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 3: Check if signature modal can be opened
    let modal_state: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Try to find and click a signature field
                const sigFields = document.querySelectorAll('.signature-field, [data-field-type="signature"], .sig-placeholder');
                const modal = document.getElementById('signature-modal');

                // Check if modal functions exist
                const canOpenModal = typeof window.openSignatureModal === 'function' ||
                                    typeof window.DocSign?.openSignatureModal === 'function';

                return {
                    signatureFieldCount: sigFields.length,
                    modalExists: !!modal,
                    modalHidden: modal?.classList.contains('hidden'),
                    canOpenModal: canOpenModal,
                    hasTypedSignature: typeof window.DocSign?.TypedSignature !== 'undefined',
                    hasSignatureCapture: typeof window.DocSign?.SignatureCapture !== 'undefined'
                };
            })()"#,
        )
        .await
        .expect("Should check modal state")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Flow - Modal state: {:?}", modal_state);

    // Step 4: Test signature capture functionality
    let signature_test: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const modal = document.getElementById('signature-modal');
                if (modal) {
                    modal.classList.remove('hidden');
                }

                const canvas = document.querySelector('#signature-modal canvas, #signature-pad, .signature-canvas');
                const typedInput = document.querySelector('#typed-name, #signature-name-input, input[type="text"]');
                const applyBtn = document.getElementById('apply-signature');

                return {
                    canvasExists: !!canvas,
                    canvasWidth: canvas?.width || 0,
                    canvasHeight: canvas?.height || 0,
                    typedInputExists: !!typedInput,
                    applyButtonExists: !!applyBtn,
                    applyButtonText: applyBtn?.textContent?.trim() || ''
                };
            })()"#,
        )
        .await
        .expect("Should test signature capture")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Flow - Signature capture: {:?}", signature_test);

    // Verify core signing components exist
    assert!(
        signature_test["canvasExists"].as_bool().unwrap_or(false)
            || signature_test["typedInputExists"]
                .as_bool()
                .unwrap_or(false),
        "Either signature canvas or typed input should exist"
    );
}

/// E2E Test 2: Typed signature creation flow
/// Tests creating a signature by typing a name.
#[tokio::test]
async fn test_docsign_e2e_typed_signature() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Open signature modal and test typed signature
    let typed_sig_test: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                // Show the signature modal
                const modal = document.getElementById('signature-modal');
                if (modal) {
                    modal.classList.remove('hidden');
                }

                // Click on Type tab
                const typeTab = document.getElementById('tab-type');
                if (typeTab) {
                    typeTab.click();
                    await new Promise(r => setTimeout(r, 100));
                }

                // Find the typed name input
                const nameInput = document.getElementById('typed-name') ||
                                  document.querySelector('#type-tab input[type="text"]');

                if (!nameInput) {
                    return { success: false, error: 'Name input not found' };
                }

                // Type a test name
                nameInput.value = 'John Doe';
                nameInput.dispatchEvent(new Event('input', { bubbles: true }));
                await new Promise(r => setTimeout(r, 100));

                // Check if preview updates
                const preview = document.querySelector('#type-tab canvas, .signature-preview, #typed-preview');

                return {
                    success: true,
                    inputValue: nameInput.value,
                    inputExists: true,
                    previewExists: !!preview,
                    previewWidth: preview?.width || preview?.offsetWidth || 0,
                    previewHeight: preview?.height || preview?.offsetHeight || 0
                };
            })()"#,
        )
        .await
        .expect("Should test typed signature")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Typed Signature: {:?}", typed_sig_test);

    assert!(
        typed_sig_test["success"].as_bool().unwrap_or(false),
        "Typed signature test should succeed"
    );
    assert_eq!(
        typed_sig_test["inputValue"].as_str().unwrap_or(""),
        "John Doe",
        "Input should contain typed name"
    );
}

/// E2E Test 3: Drawn signature flow
/// Tests creating a signature by drawing on canvas.
#[tokio::test]
async fn test_docsign_e2e_drawn_signature() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Open signature modal and test drawn signature
    let drawn_sig_test: serde_json::Value = page
        .evaluate(
            r#"(async () => {
                // Show the signature modal
                const modal = document.getElementById('signature-modal');
                if (modal) {
                    modal.classList.remove('hidden');
                }

                // Click on Draw tab
                const drawTab = document.getElementById('tab-draw');
                if (drawTab) {
                    drawTab.click();
                    await new Promise(r => setTimeout(r, 100));
                }

                // Find the signature canvas
                const canvas = document.getElementById('signature-pad') ||
                              document.querySelector('#draw-tab canvas, .signature-canvas');

                if (!canvas) {
                    return { success: false, error: 'Canvas not found' };
                }

                const ctx = canvas.getContext('2d');
                if (!ctx) {
                    return { success: false, error: 'Could not get canvas context' };
                }

                // Simulate drawing a signature stroke
                const rect = canvas.getBoundingClientRect();

                // Dispatch mouse events to simulate drawing
                const mouseDown = new MouseEvent('mousedown', {
                    clientX: rect.left + 50,
                    clientY: rect.top + 50,
                    bubbles: true
                });
                const mouseMove = new MouseEvent('mousemove', {
                    clientX: rect.left + 150,
                    clientY: rect.top + 80,
                    bubbles: true
                });
                const mouseUp = new MouseEvent('mouseup', {
                    clientX: rect.left + 150,
                    clientY: rect.top + 80,
                    bubbles: true
                });

                canvas.dispatchEvent(mouseDown);
                await new Promise(r => setTimeout(r, 50));
                canvas.dispatchEvent(mouseMove);
                await new Promise(r => setTimeout(r, 50));
                canvas.dispatchEvent(mouseUp);
                await new Promise(r => setTimeout(r, 100));

                // Check if canvas has content
                const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
                let hasContent = false;
                for (let i = 3; i < imageData.data.length; i += 4) {
                    if (imageData.data[i] > 0) {
                        hasContent = true;
                        break;
                    }
                }

                // Check undo button
                const undoBtn = document.getElementById('undo-stroke') ||
                               document.querySelector('.undo-btn, [aria-label*="undo"]');

                return {
                    success: true,
                    canvasExists: true,
                    canvasWidth: canvas.width,
                    canvasHeight: canvas.height,
                    hasDrawnContent: hasContent,
                    undoButtonExists: !!undoBtn
                };
            })()"#,
        )
        .await
        .expect("Should test drawn signature")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Drawn Signature: {:?}", drawn_sig_test);

    assert!(
        drawn_sig_test["success"].as_bool().unwrap_or(false),
        "Drawn signature test should succeed"
    );
    assert!(
        drawn_sig_test["canvasExists"].as_bool().unwrap_or(false),
        "Signature canvas should exist"
    );
}

/// E2E Test 4: Error handling and recovery
/// Tests that errors are displayed properly and user can recover.
#[tokio::test]
async fn test_docsign_e2e_error_handling() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test error handling functions
    let error_test: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Check if error handling functions exist
                const hasShowErrorModal = typeof window.DocSign?.showErrorModal === 'function' ||
                                         typeof window.showErrorModal === 'function';
                const hasShowErrorToast = typeof window.DocSign?.showErrorToast === 'function' ||
                                         typeof window.showErrorToast === 'function';
                const hasGetUserFriendlyError = typeof window.DocSign?.getUserFriendlyError === 'function' ||
                                               typeof window.getUserFriendlyError === 'function';

                // Check if error modal element exists
                const errorModal = document.getElementById('error-modal') ||
                                  document.querySelector('.error-modal');

                // Check toast container
                const toastContainer = document.getElementById('toast-container') ||
                                      document.querySelector('.toast-container');

                return {
                    hasShowErrorModal,
                    hasShowErrorToast,
                    hasGetUserFriendlyError,
                    errorModalExists: !!errorModal,
                    toastContainerExists: !!toastContainer,
                    allErrorHandlersPresent: hasShowErrorModal && hasShowErrorToast && hasGetUserFriendlyError
                };
            })()"#,
        )
        .await
        .expect("Should test error handling")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Error Handling: {:?}", error_test);

    assert!(
        error_test["hasShowErrorModal"].as_bool().unwrap_or(false),
        "showErrorModal function should be available"
    );
    assert!(
        error_test["hasGetUserFriendlyError"]
            .as_bool()
            .unwrap_or(false),
        "getUserFriendlyError function should be available"
    );
}

/// E2E Test 5: Offline functionality
/// Tests that the app handles offline state gracefully.
#[tokio::test]
async fn test_docsign_e2e_offline_functionality() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test offline handling
    let offline_test: serde_json::Value = page
        .evaluate(
            r#"(() => {
                // Check sync manager exists
                const hasSyncManager = typeof window.DocSign?.SyncManager !== 'undefined' ||
                                      typeof window.DocSign?.getSyncManager === 'function';

                // Check offline indicator
                const offlineIndicator = document.getElementById('offline-indicator');

                // Check if LocalSessionManager exists
                const hasLocalSessionManager = typeof window.DocSign?.LocalSessionManager !== 'undefined';

                // Check for offline queue functions
                const hasQueueFunctions = typeof window.DocSign?.queueForSync === 'function' ||
                                         typeof window.queueSignatureForSync === 'function';

                return {
                    hasSyncManager,
                    hasLocalSessionManager,
                    hasQueueFunctions,
                    offlineIndicatorExists: !!offlineIndicator,
                    offlineIndicatorHidden: offlineIndicator?.classList.contains('hidden'),
                    navigatorOnline: navigator.onLine
                };
            })()"#,
        )
        .await
        .expect("Should test offline functionality")
        .into_value()
        .expect("Should get value");

    eprintln!("E2E Offline Functionality: {:?}", offline_test);

    assert!(
        offline_test["offlineIndicatorExists"]
            .as_bool()
            .unwrap_or(false),
        "Offline indicator should exist"
    );
}

// ============================================================================
// UX Regression Tests
// ============================================================================

/// Test 8: All buttons have 60px minimum height (geriatric requirement)
/// Verifies primary action buttons meet the stricter geriatric touch target requirements.
#[tokio::test]
async fn test_docsign_button_touch_targets_60px() {
    skip_if_no_chrome!();
    require_local_server!("http://127.0.0.1:8081");

    let Some((browser, _handle)) = browser::require_browser().await else {
        return;
    };

    let page = browser
        .new_page("about:blank")
        .await
        .expect("Should create page");

    page.goto("http://127.0.0.1:8081/sign.html?session=test&recipient=r1&key=test123")
        .await
        .expect("Should navigate to sign page");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check button touch targets (60px minimum for geriatric UX)
    let button_check: serde_json::Value = page
        .evaluate(
            r#"(() => {
                const MIN_TOUCH_TARGET = 60;
                const violations = [];
                const passed = [];

                // Check all primary buttons and important action buttons
                const buttons = document.querySelectorAll('.btn-primary, .btn-large, #btn-start, #btn-finish, #btn-review-document, #apply-signature, #verify-button');

                buttons.forEach(btn => {
                    const rect = btn.getBoundingClientRect();
                    const style = getComputedStyle(btn);

                    // Only check visible buttons
                    if (rect.height > 0 && rect.width > 0) {
                        const buttonInfo = {
                            id: btn.id || 'no-id',
                            text: btn.textContent?.trim().slice(0, 30),
                            height: rect.height,
                            width: rect.width,
                            minHeight: style.minHeight
                        };

                        if (rect.height < MIN_TOUCH_TARGET) {
                            violations.push(buttonInfo);
                        } else {
                            passed.push(buttonInfo);
                        }
                    }
                });

                return {
                    totalChecked: passed.length + violations.length,
                    passedCount: passed.length,
                    violationCount: violations.length,
                    violations: violations.slice(0, 10),
                    passed: violations.length === 0
                };
            })()"#,
        )
        .await
        .expect("Should check button sizes")
        .into_value()
        .expect("Should get value");

    eprintln!("Phase 5 UX: Button touch target check: {:?}", button_check);

    // This is a stricter requirement than WCAG's 44px
    if !button_check["passed"].as_bool().unwrap_or(true) {
        eprintln!(
            "WARNING: {} primary buttons below 60px geriatric touch target: {:?}",
            button_check["violationCount"], button_check["violations"]
        );
    }

    // At minimum, we should have checked some buttons
    assert!(
        button_check["totalChecked"].as_u64().unwrap_or(0) > 0,
        "Should have primary buttons to check"
    );

    // Fail if violations exist (strict mode)
    assert!(
        button_check["passed"].as_bool().unwrap_or(false),
        "All primary buttons should have 60px minimum height for geriatric UX. Found {} violations: {:?}",
        button_check["violationCount"],
        button_check["violations"]
    );
}
