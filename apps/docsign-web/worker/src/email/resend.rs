//! Direct Resend API client
//!
//! Sends emails directly via Resend API (https://api.resend.com/emails).
//! Replaces the Mayo gateway with direct integration.

use serde::{Deserialize, Serialize};
use worker::{console_log, Fetch, Headers, Method, Request, RequestInit, Result};

use super::{EmailSendRequest, EmailSendResult};

/// Resend API configuration
pub struct ResendConfig {
    /// Resend API key (re_xxxxxxxxxx)
    pub api_key: String,
    /// From address for emails
    pub from_address: String,
}

/// Resend API request payload
#[derive(Debug, Serialize)]
struct ResendPayload<'a> {
    from: &'a str,
    to: &'a [String],
    subject: &'a str,
    html: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<ResendTag<'a>>,
}

/// Resend tag for tracking
#[derive(Debug, Serialize)]
struct ResendTag<'a> {
    name: &'a str,
    value: &'a str,
}

/// Resend API success response
#[derive(Debug, Deserialize)]
struct ResendSuccessResponse {
    id: String,
}

/// Resend API error response
#[derive(Debug, Deserialize)]
struct ResendErrorResponse {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    statusCode: Option<u16>,
}

/// Resend API endpoint
const RESEND_API_URL: &str = "https://api.resend.com/emails";

/// Send email via Resend API
///
/// Makes a direct HTTP POST to Resend's API endpoint.
///
/// # Arguments
/// * `config` - Resend configuration (API key, from address)
/// * `request` - The email to send
///
/// # Returns
/// * `Ok(EmailSendResult)` - Result with message ID or error
pub async fn send_via_resend(
    config: &ResendConfig,
    request: &EmailSendRequest,
) -> Result<EmailSendResult> {
    // Convert tags to Resend format
    let tags: Vec<ResendTag> = request
        .tags
        .iter()
        .map(|(name, value)| ResendTag {
            name: name.as_str(),
            value: value.as_str(),
        })
        .collect();

    let payload = ResendPayload {
        from: &config.from_address,
        to: &request.to,
        subject: &request.subject,
        html: &request.html,
        text: request.text.as_deref(),
        reply_to: request.reply_to.as_deref(),
        tags,
    };

    let payload_json = serde_json::to_string(&payload)
        .map_err(|e| worker::Error::RustError(format!("JSON serialize error: {}", e)))?;

    // Build request headers
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Authorization", &format!("Bearer {}", config.api_key))?;

    // Build POST request
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(payload_json.into()));

    let http_request = Request::new_with_init(RESEND_API_URL, &init)?;

    console_log!("Sending email via Resend to: {:?}", request.to);

    // Send request
    match Fetch::Request(http_request).send().await {
        Ok(mut response) => {
            let status = response.status_code();

            if status == 200 {
                // Success
                match response.json::<ResendSuccessResponse>().await {
                    Ok(success) => {
                        console_log!("Email sent via Resend: id={}", success.id);
                        Ok(EmailSendResult::success(success.id))
                    }
                    Err(e) => {
                        // Response parse error but email likely sent
                        console_log!("Warning: Resend response parse error: {}", e);
                        Ok(EmailSendResult::success("resend:unknown".to_string()))
                    }
                }
            } else {
                // Error response
                let error_text = response.text().await.unwrap_or_default();
                console_log!("Resend error ({}): {}", status, error_text);

                let error_message =
                    if let Ok(err) = serde_json::from_str::<ResendErrorResponse>(&error_text) {
                        err.message
                            .or(err.name)
                            .unwrap_or_else(|| format!("HTTP {}", status))
                    } else {
                        format!("HTTP {}: {}", status, error_text)
                    };

                // User-friendly error messages
                let user_message = match status {
                    401 => "Email service authentication failed.".to_string(),
                    403 => "Email service access denied.".to_string(),
                    422 => format!("Invalid email request: {}", error_message),
                    429 => "Email rate limit exceeded. Please try again later.".to_string(),
                    500..=599 => "Email service temporarily unavailable.".to_string(),
                    _ => error_message,
                };

                Ok(EmailSendResult::error(user_message))
            }
        }
        Err(e) => {
            console_log!("Resend request failed: {}", e);
            Ok(EmailSendResult::error(
                "Failed to connect to email service.",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resend_payload_serialization() {
        let payload = ResendPayload {
            from: "sender@example.com",
            to: &vec!["recipient@example.com".to_string()],
            subject: "Test Subject",
            html: "<p>Hello</p>",
            text: None,
            reply_to: None,
            tags: vec![],
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("sender@example.com"));
        assert!(json.contains("recipient@example.com"));
        assert!(json.contains("Test Subject"));
        // text and reply_to should be omitted (skip_serializing_if)
        assert!(!json.contains("\"text\""));
        assert!(!json.contains("\"reply_to\""));
        // Empty tags should be omitted
        assert!(!json.contains("\"tags\""));
    }

    #[test]
    fn test_resend_payload_with_optional_fields() {
        let payload = ResendPayload {
            from: "sender@example.com",
            to: &vec!["recipient@example.com".to_string()],
            subject: "Test",
            html: "<p>Hello</p>",
            text: Some("Hello"),
            reply_to: Some("reply@example.com"),
            tags: vec![
                ResendTag {
                    name: "type",
                    value: "test",
                },
                ResendTag {
                    name: "campaign",
                    value: "onboarding",
                },
            ],
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"text\":\"Hello\""));
        assert!(json.contains("\"reply_to\":\"reply@example.com\""));
        assert!(json.contains("\"tags\""));
        assert!(json.contains("\"type\""));
        assert!(json.contains("\"campaign\""));
    }

    #[test]
    fn test_resend_error_response_parsing() {
        let error_json =
            r#"{"statusCode":422,"message":"Invalid email address","name":"validation_error"}"#;
        let error: ResendErrorResponse = serde_json::from_str(error_json).unwrap();
        assert_eq!(error.statusCode, Some(422));
        assert_eq!(error.message.as_deref(), Some("Invalid email address"));
        assert_eq!(error.name.as_deref(), Some("validation_error"));
    }
}
