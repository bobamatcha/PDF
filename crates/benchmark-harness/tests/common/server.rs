//! Local server helpers

/// Check if a local server is available
pub async fn is_server_available(url: &str) -> bool {
    match reqwest::get(url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Check if the correct app is running by looking for expected content in response
#[allow(dead_code)]
pub async fn is_correct_app(url: &str, expected_marker: &str) -> bool {
    match reqwest::get(url).await {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                body.contains(expected_marker)
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Macro to skip test if local server isn't running
#[macro_export]
macro_rules! require_local_server {
    ($url:expr) => {{
        if !server::is_server_available($url).await {
            eprintln!("Skipping: Local server not running at {}", $url);
            eprintln!("  To run these tests, start the server with:");
            eprintln!("    trunk serve --port 8080  (for agentpdf)");
            eprintln!("    trunk serve --port 8081  (for docsign)");
            eprintln!("    trunk serve --port 8082  (for pdfjoin)");
            return;
        }
    }};
}

/// Macro to skip test if the wrong app is running on the port
#[macro_export]
macro_rules! require_correct_app {
    ($url:expr, $marker:expr, $app_name:expr) => {{
        if !server::is_correct_app($url, $marker).await {
            eprintln!(
                "Skipping: {} is not running at {} (different app detected)",
                $app_name, $url
            );
            return;
        }
    }};
}
