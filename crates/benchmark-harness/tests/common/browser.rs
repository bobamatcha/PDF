//! Browser automation helpers

use anyhow::Result;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::StreamExt;
use std::time::Duration;

/// Check if browser tests should be skipped (when Chrome isn't available)
pub fn should_skip() -> bool {
    std::env::var("SKIP_BROWSER_TESTS").is_ok()
}

/// Macro to skip test if Chrome isn't available
#[macro_export]
macro_rules! skip_if_no_chrome {
    () => {
        if browser::should_skip() {
            eprintln!("Skipping test: SKIP_BROWSER_TESTS is set");
            return;
        }
    };
}

/// Find Chrome for Testing installed by Puppeteer
pub fn find_chrome_for_testing() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let puppeteer_cache = std::path::Path::new(&home).join(".cache/puppeteer/chrome");

    if puppeteer_cache.exists() {
        if let Ok(entries) = std::fs::read_dir(&puppeteer_cache) {
            let mut versions: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();
            versions.sort_by_key(|v| std::cmp::Reverse(v.path()));

            for version_dir in versions {
                // macOS arm64
                let chrome_app = version_dir.path().join(
                    "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
                );
                if chrome_app.exists() {
                    return Some(chrome_app);
                }
                // macOS x64
                let chrome_app_x64 = version_dir.path().join(
                    "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
                );
                if chrome_app_x64.exists() {
                    return Some(chrome_app_x64);
                }
                // Linux
                let chrome_linux = version_dir.path().join("chrome-linux64/chrome");
                if chrome_linux.exists() {
                    return Some(chrome_linux);
                }
            }
        }
    }
    None
}

/// Helper to create a headless browser for testing
pub async fn create_test_browser() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static BROWSER_ID: AtomicU64 = AtomicU64::new(0);

    let mut builder = BrowserConfig::builder();

    // Use Chrome for Testing if available (same as Puppeteer)
    if let Some(chrome_path) = find_chrome_for_testing() {
        eprintln!("Using Chrome for Testing: {}", chrome_path.display());
        builder = builder.chrome_executable(chrome_path);
    }

    // Use unique user data directory to avoid conflicts when running tests in parallel
    // Include PID + counter + timestamp to ensure uniqueness across test binaries
    let browser_id = BROWSER_ID.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let user_data_dir = std::env::temp_dir().join(format!(
        "benchmark-harness-{}-{}-{}",
        pid, browser_id, timestamp
    ));

    // Clean up any stale directory (shouldn't exist with timestamp, but just in case)
    if user_data_dir.exists() {
        let _ = std::fs::remove_dir_all(&user_data_dir);
    }

    builder = builder.user_data_dir(user_data_dir);

    let config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?;

    let (browser, mut handler) = Browser::launch(config).await?;

    let handle = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(e) = event {
                eprintln!("Browser handler error: {:?}", e);
                break;
            }
        }
    });

    // Give browser a moment to fully initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((browser, handle))
}

/// Try to create browser, skip test if Chrome not found
pub async fn require_browser() -> Option<(Browser, tokio::task::JoinHandle<()>)> {
    match create_test_browser().await {
        Ok(browser) => Some(browser),
        Err(e) => {
            if e.to_string().contains("Could not auto detect") {
                eprintln!("Skipping: Chrome not installed ({})", e);
                None
            } else {
                panic!("Unexpected browser error: {}", e);
            }
        }
    }
}

/// Clear all browser storage (localStorage, sessionStorage, IndexedDB, cookies) for clean test context
#[allow(dead_code)]
pub async fn clear_browser_storage(page: &Page) -> Result<()> {
    page.evaluate(
        r#"(async () => {
            // Clear localStorage
            try { localStorage.clear(); } catch(e) {}

            // Clear sessionStorage
            try { sessionStorage.clear(); } catch(e) {}

            // Clear IndexedDB databases
            try {
                const databases = await indexedDB.databases?.() || [];
                for (const db of databases) {
                    if (db.name) {
                        indexedDB.deleteDatabase(db.name);
                    }
                }
            } catch(e) {}

            // Clear Cache Storage
            try {
                const cacheNames = await caches.keys();
                for (const name of cacheNames) {
                    await caches.delete(name);
                }
            } catch(e) {}

            return true;
        })()"#,
    )
    .await?;
    Ok(())
}

/// Navigate to URL with clean context - clears storage before loading
#[allow(dead_code)]
pub async fn navigate_clean(page: &Page, url: &str) -> Result<()> {
    // First navigate to about:blank to get a clean page
    page.goto("about:blank").await?;

    // Clear any existing storage
    clear_browser_storage(page).await?;

    // Now navigate to the actual URL
    page.goto(url).await?;

    Ok(())
}

/// Macro to clear storage at origin before test
#[macro_export]
macro_rules! clear_storage {
    ($page:expr, $url:expr) => {{
        // Navigate to origin first to clear its storage
        $page.goto($url).await.expect("Should navigate");
        browser::clear_browser_storage(&$page)
            .await
            .expect("Should clear storage");
        // Navigate away and back to reset app state
        $page.goto("about:blank").await.expect("Should navigate");
        $page.goto($url).await.expect("Should navigate");
    }};
}
