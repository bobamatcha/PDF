//! AWS Lambda handler for the email proxy
//!
//! This Lambda function handles:
//! - POST /send - Send an email
//! - POST /send/template - Send using a DocSign template
//! - POST /notifications - Process SNS bounce/complaint notifications
//! - GET /health - Health check
//! - GET /metrics - Get reputation metrics
//!
//! ## Deployment
//!
//! ```bash
//! # Install cargo-lambda
//! cargo install cargo-lambda
//!
//! # Build for ARM64 (30% cheaper)
//! cargo lambda build --release --arm64
//!
//! # Deploy
//! cargo lambda deploy --iam-role arn:aws:iam::ACCOUNT:role/email-proxy-lambda
//! ```

use email_proxy::{
    deliverability::{scan_content_for_spam, SesNotification},
    ses::SesSender,
    types::{EmailTemplate, SendEmailRequest},
    EmailProxyConfig,
};
use lambda_http::{http::StatusCode, run, service_fn, Body, Error, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{error, info, instrument, warn};

/// Global SES sender (initialized once)
static SES_SENDER: OnceCell<Arc<SesSender>> = OnceCell::const_new();

/// Get or initialize the SES sender
async fn get_sender() -> Arc<SesSender> {
    SES_SENDER
        .get_or_init(|| async { Arc::new(SesSender::new().await) })
        .await
        .clone()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing with CloudWatch-optimized settings
    // See: https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false) // CloudWatch doesn't support ANSI colors
        .with_current_span(false) // Reduce duplicate info in logs
        .without_time() // CloudWatch adds ingestion time
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("email_proxy=info".parse().unwrap()),
        )
        .init();

    info!(
        version = email_proxy::VERSION,
        "Starting email proxy Lambda"
    );

    // Run the Lambda service
    run(service_fn(handler)).await
}

/// Main Lambda handler
#[instrument(skip(event), fields(method = %event.method(), path = %event.uri().path()))]
async fn handler(event: Request) -> Result<Response<Body>, Error> {
    let method = event.method().clone();
    let path = event.uri().path().to_string();

    let response = match (method.as_str(), path.as_str()) {
        ("POST", "/send") => handle_send(event).await,
        ("POST", "/send/template") => handle_send_template(event).await,
        ("POST", "/notifications") => handle_sns_notification(event).await,
        ("GET", "/health") => handle_health().await,
        ("GET", "/metrics") => handle_metrics().await,
        ("POST", "/validate") => handle_validate(event).await,
        ("OPTIONS", _) => handle_cors_preflight(),
        _ => {
            warn!(method = %method, path = %path, "Route not found");
            Ok(json_response(
                StatusCode::NOT_FOUND,
                json!({ "error": "Not found" }),
            ))
        }
    };

    // Add CORS headers to all responses
    response.map(|mut resp| {
        let headers = resp.headers_mut();
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert(
            "Access-Control-Allow-Methods",
            "GET, POST, OPTIONS".parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-API-Key".parse().unwrap(),
        );
        resp
    })
}

/// Handle POST /send - Send an email
async fn handle_send(event: Request) -> Result<Response<Body>, Error> {
    // Parse request body
    let body = event.body();
    let request: SendEmailRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(req) => req,
        Err(e) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("Invalid request: {}", e) }),
            ));
        }
    };

    // Check for spam content (warning only)
    if let Some(ref html) = request.html {
        let scan = scan_content_for_spam(&request.subject, html);
        if scan.is_likely_spam {
            warn!(
                score = scan.score,
                triggers = ?scan.triggers,
                "Content flagged as potential spam"
            );
        }
    }

    // Send email
    let sender = get_sender().await;
    match sender.send(request).await {
        Ok(response) => Ok(json_response(StatusCode::OK, response)),
        Err(e) => {
            error!(error = %e, "Failed to send email");
            let status = match &e {
                email_proxy::ses::SesError::Validation(_) => StatusCode::BAD_REQUEST,
                email_proxy::ses::SesError::Suppressed { .. } => StatusCode::UNPROCESSABLE_ENTITY,
                email_proxy::ses::SesError::Blocked(_) => StatusCode::TOO_MANY_REQUESTS,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Ok(json_response(status, json!({ "error": e.to_string() })))
        }
    }
}

/// Request for template-based sending
#[derive(Debug, Deserialize)]
struct TemplateRequest {
    template: EmailTemplate,
    #[serde(default)]
    from: Option<String>,
}

/// Handle POST /send/template - Send using a DocSign template
async fn handle_send_template(event: Request) -> Result<Response<Body>, Error> {
    let body = event.body();
    let req: TemplateRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(req) => req,
        Err(e) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("Invalid request: {}", e) }),
            ));
        }
    };

    let config = EmailProxyConfig::from_env();
    let from = req.from.as_deref().unwrap_or(&config.default_from);

    // Convert template to email request
    let email_request = req.template.to_request(from);

    // Send email
    let sender = get_sender().await;
    match sender.send(email_request).await {
        Ok(response) => Ok(json_response(StatusCode::OK, response)),
        Err(e) => {
            error!(error = %e, "Failed to send template email");
            Ok(json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": e.to_string() }),
            ))
        }
    }
}

/// SNS notification wrapper
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SnsMessage {
    #[serde(rename = "Type")]
    message_type: String,
    message: String,
    #[serde(default)]
    subscribe_url: Option<String>,
}

/// Handle POST /notifications - Process SNS bounce/complaint notifications
async fn handle_sns_notification(event: Request) -> Result<Response<Body>, Error> {
    let body = event.body();

    // SNS sends JSON directly or wrapped
    let notification: SesNotification =
        if let Ok(sns_msg) = serde_json::from_slice::<SnsMessage>(body.as_ref()) {
            // Handle subscription confirmation
            if sns_msg.message_type == "SubscriptionConfirmation" {
                info!(
                    subscribe_url = ?sns_msg.subscribe_url,
                    "SNS subscription confirmation received"
                );
                // In production, automatically confirm by fetching the subscribe URL
                return Ok(json_response(
                    StatusCode::OK,
                    json!({ "message": "Subscription confirmation received" }),
                ));
            }

            // Parse the inner message
            match serde_json::from_str(&sns_msg.message) {
                Ok(n) => n,
                Err(e) => {
                    error!(error = %e, "Failed to parse SNS message content");
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        json!({ "error": "Invalid notification format" }),
                    ));
                }
            }
        } else {
            // Try parsing directly (for testing)
            match serde_json::from_slice(body.as_ref()) {
                Ok(n) => n,
                Err(e) => {
                    error!(error = %e, "Failed to parse notification");
                    return Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        json!({ "error": "Invalid notification format" }),
                    ));
                }
            }
        };

    // Process the notification
    let sender = get_sender().await;
    sender.process_notification(notification).await;

    info!("Processed SES notification");
    Ok(json_response(
        StatusCode::OK,
        json!({ "message": "Notification processed" }),
    ))
}

/// Handle GET /health - Health check
async fn handle_health() -> Result<Response<Body>, Error> {
    let sender = get_sender().await;
    let metrics = sender.deliverability().get_metrics().await;
    let warm_up = sender.deliverability().warm_up_status();

    Ok(json_response(
        StatusCode::OK,
        json!({
            "status": "healthy",
            "version": email_proxy::VERSION,
            "warm_up": warm_up,
            "health_score": metrics.health_score(),
        }),
    ))
}

/// Handle GET /metrics - Get reputation metrics
async fn handle_metrics() -> Result<Response<Body>, Error> {
    let sender = get_sender().await;
    let metrics = sender.deliverability().get_metrics().await;
    let warm_up = sender.deliverability().warm_up_status();

    Ok(json_response(
        StatusCode::OK,
        json!({
            "metrics": {
                "total_sent": metrics.total_sent,
                "delivered": metrics.delivered,
                "hard_bounces": metrics.hard_bounces,
                "soft_bounces": metrics.soft_bounces,
                "complaints": metrics.complaints,
                "bounce_rate": format!("{:.2}%", metrics.bounce_rate()),
                "complaint_rate": format!("{:.4}%", metrics.complaint_rate()),
                "health_score": metrics.health_score(),
                "is_healthy": metrics.is_healthy(),
            },
            "warm_up": warm_up,
        }),
    ))
}

/// Validation request
#[derive(Debug, Deserialize)]
struct ValidateRequest {
    email: SendEmailRequest,
    #[serde(default)]
    scan_content: bool,
}

/// Handle POST /validate - Validate an email without sending
async fn handle_validate(event: Request) -> Result<Response<Body>, Error> {
    let body = event.body();
    let req: ValidateRequest = match serde_json::from_slice(body.as_ref()) {
        Ok(req) => req,
        Err(e) => {
            return Ok(json_response(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("Invalid request: {}", e) }),
            ));
        }
    };

    // Validate the email
    let validation_result = req.email.validate();

    // Optionally scan for spam
    let spam_scan = if req.scan_content {
        req.email.html.as_ref().map(|html| {
            let scan = scan_content_for_spam(&req.email.subject, html);
            json!({
                "score": scan.score,
                "is_likely_spam": scan.is_likely_spam,
                "triggers": scan.triggers,
            })
        })
    } else {
        None
    };

    // Check suppression for recipients
    let sender = get_sender().await;
    let mut suppressed = vec![];
    for to in &req.email.to {
        if let Some(entry) = sender.deliverability().is_suppressed(to).await {
            suppressed.push(json!({
                "email": to,
                "reason": format!("{:?}", entry.reason),
                "added_at": entry.added_at.to_rfc3339(),
            }));
        }
    }

    Ok(json_response(
        StatusCode::OK,
        json!({
            "valid": validation_result.is_ok(),
            "validation_error": validation_result.err().map(|e| e.to_string()),
            "spam_scan": spam_scan,
            "suppressed_recipients": suppressed,
        }),
    ))
}

/// Handle CORS preflight
fn handle_cors_preflight() -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-API-Key",
        )
        .header("Access-Control-Max-Age", "86400")
        .body(Body::Empty)
        .unwrap())
}

/// Create a JSON response
fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}
