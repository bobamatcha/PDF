//! Local server helpers

/// Check if a local server is available
pub async fn is_server_available(url: &str) -> bool {
    match reqwest::get(url).await {
        Ok(resp) => resp.status().is_success(),
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
