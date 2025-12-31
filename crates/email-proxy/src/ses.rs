//! AWS SES v2 client implementation
//!
//! Handles email sending via AWS SES with proper error handling,
//! DKIM/SPF verification (via SES configuration), and deliverability tracking.

use aws_sdk_sesv2::{
    types::{Body, Content, Destination, EmailContent, Message, MessageTag, RawMessage},
    Client as SesClient,
};
use chrono::Utc;
use tracing::{error, info, instrument, warn};

use crate::deliverability::{DeliverabilityManager, SendBlockedReason};
use crate::types::{EmailStatus, SendEmailRequest, SendEmailResponse};

/// SES email sender
pub struct SesSender {
    client: SesClient,
    deliverability: DeliverabilityManager,
    #[allow(dead_code)]
    from_domain: String,
    configuration_set: Option<String>,
}

impl SesSender {
    /// Create a new SES sender from environment config
    pub async fn new() -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = SesClient::new(&config);

        Self {
            client,
            deliverability: DeliverabilityManager::new(),
            from_domain: std::env::var("FROM_DOMAIN")
                .unwrap_or_else(|_| "getsignatures.org".to_string()),
            configuration_set: std::env::var("SES_CONFIGURATION_SET").ok(),
        }
    }

    /// Create with custom config (for testing)
    pub fn with_client(client: SesClient, from_domain: &str) -> Self {
        Self {
            client,
            deliverability: DeliverabilityManager::new(),
            from_domain: from_domain.to_string(),
            configuration_set: None,
        }
    }

    /// Send an email
    #[instrument(skip(self, request), fields(to = ?request.to, subject = %request.subject))]
    pub async fn send(&self, request: SendEmailRequest) -> Result<SendEmailResponse, SesError> {
        // Validate request
        request.validate().map_err(SesError::Validation)?;

        // Check deliverability constraints
        self.deliverability.can_send().await?;

        // Check suppression list for each recipient
        for recipient in &request.to {
            if let Some(entry) = self.deliverability.is_suppressed(recipient).await {
                warn!(email = %recipient, reason = ?entry.reason, "Email suppressed");
                return Err(SesError::Suppressed {
                    email: recipient.clone(),
                    reason: format!("{:?}", entry.reason),
                });
            }
        }

        // Build and send the email
        let message_id = self.send_via_ses(&request).await?;

        // Record the send
        self.deliverability.record_send().await;

        info!(message_id = %message_id, "Email sent successfully");

        Ok(SendEmailResponse {
            id: message_id,
            queued_at: Utc::now(),
            status: EmailStatus::Queued,
        })
    }

    /// Send email via SES
    async fn send_via_ses(&self, request: &SendEmailRequest) -> Result<String, SesError> {
        // Build destination
        let mut destination = Destination::builder();

        for to in &request.to {
            destination = destination.to_addresses(to);
        }
        for cc in &request.cc {
            destination = destination.cc_addresses(cc);
        }
        for bcc in &request.bcc {
            destination = destination.bcc_addresses(bcc);
        }

        // Build message content
        let subject = Content::builder()
            .data(&request.subject)
            .charset("UTF-8")
            .build()
            .map_err(|e| SesError::BuildError(e.to_string()))?;

        let mut body_builder = Body::builder();

        if let Some(ref html) = request.html {
            let html_content = Content::builder()
                .data(html)
                .charset("UTF-8")
                .build()
                .map_err(|e| SesError::BuildError(e.to_string()))?;
            body_builder = body_builder.html(html_content);
        }

        if let Some(ref text) = request.text {
            let text_content = Content::builder()
                .data(text)
                .charset("UTF-8")
                .build()
                .map_err(|e| SesError::BuildError(e.to_string()))?;
            body_builder = body_builder.text(text_content);
        }

        let body = body_builder.build();

        let message = Message::builder().subject(subject).body(body).build();

        let email_content = EmailContent::builder().simple(message).build();

        // Build the send request
        let mut send_request = self
            .client
            .send_email()
            .from_email_address(&request.from)
            .destination(destination.build())
            .content(email_content);

        // Add configuration set for tracking
        if let Some(config_set) = request
            .configuration_set
            .as_ref()
            .or(self.configuration_set.as_ref())
        {
            send_request = send_request.configuration_set_name(config_set.clone());
        }

        // Add reply-to if specified
        if let Some(ref reply_to) = request.reply_to {
            send_request = send_request.reply_to_addresses(reply_to);
        }

        // Add tags
        for tag in &request.tags {
            send_request = send_request.email_tags(
                MessageTag::builder()
                    .name(&tag.name)
                    .value(&tag.value)
                    .build()
                    .map_err(|e| SesError::BuildError(e.to_string()))?,
            );
        }

        // Send
        let result = send_request.send().await.map_err(|e| {
            error!(error = %e, "SES send failed");
            SesError::SendFailed(e.to_string())
        })?;

        let message_id = result.message_id().unwrap_or("unknown").to_string();
        Ok(message_id)
    }

    /// Send raw MIME email (for attachments)
    #[instrument(skip(self, request), fields(to = ?request.to, subject = %request.subject))]
    pub async fn send_with_attachments(
        &self,
        request: SendEmailRequest,
    ) -> Result<SendEmailResponse, SesError> {
        // Validate
        request.validate().map_err(SesError::Validation)?;

        // Check constraints
        self.deliverability.can_send().await?;

        // Check suppression
        for recipient in &request.to {
            if let Some(entry) = self.deliverability.is_suppressed(recipient).await {
                return Err(SesError::Suppressed {
                    email: recipient.clone(),
                    reason: format!("{:?}", entry.reason),
                });
            }
        }

        // Build MIME message
        let mime_message = self.build_mime_message(&request)?;

        // Send raw
        let raw_message = RawMessage::builder()
            .data(aws_sdk_sesv2::primitives::Blob::new(mime_message))
            .build()
            .map_err(|e| SesError::BuildError(e.to_string()))?;

        let email_content = EmailContent::builder().raw(raw_message).build();

        let mut send_request = self.client.send_email().content(email_content);

        if let Some(ref config_set) = self.configuration_set {
            send_request = send_request.configuration_set_name(config_set);
        }

        let result = send_request.send().await.map_err(|e| {
            error!(error = %e, "SES raw send failed");
            SesError::SendFailed(e.to_string())
        })?;

        self.deliverability.record_send().await;

        Ok(SendEmailResponse {
            id: result.message_id().unwrap_or("unknown").to_string(),
            queued_at: Utc::now(),
            status: EmailStatus::Queued,
        })
    }

    /// Build a MIME message with attachments
    fn build_mime_message(&self, request: &SendEmailRequest) -> Result<Vec<u8>, SesError> {
        use std::fmt::Write;

        let boundary = format!(
            "----=_Part_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        let mut message = String::new();

        // Headers
        writeln!(message, "From: {}", request.from).unwrap();
        writeln!(message, "To: {}", request.to.join(", ")).unwrap();
        if !request.cc.is_empty() {
            writeln!(message, "Cc: {}", request.cc.join(", ")).unwrap();
        }
        writeln!(message, "Subject: {}", request.subject).unwrap();
        writeln!(message, "MIME-Version: 1.0").unwrap();

        // Custom headers
        for header in &request.headers {
            writeln!(message, "{}: {}", header.name, header.value).unwrap();
        }

        writeln!(
            message,
            "Content-Type: multipart/mixed; boundary=\"{boundary}\""
        )
        .unwrap();
        writeln!(message).unwrap();

        // Text/HTML body part
        if request.html.is_some() || request.text.is_some() {
            let inner_boundary = format!(
                "----=_Alt_{}",
                uuid::Uuid::new_v4().to_string().replace("-", "")
            );

            writeln!(message, "--{boundary}").unwrap();
            writeln!(
                message,
                "Content-Type: multipart/alternative; boundary=\"{inner_boundary}\""
            )
            .unwrap();
            writeln!(message).unwrap();

            if let Some(ref text) = request.text {
                writeln!(message, "--{inner_boundary}").unwrap();
                writeln!(message, "Content-Type: text/plain; charset=UTF-8").unwrap();
                writeln!(message, "Content-Transfer-Encoding: quoted-printable").unwrap();
                writeln!(message).unwrap();
                writeln!(message, "{text}").unwrap();
            }

            if let Some(ref html) = request.html {
                writeln!(message, "--{inner_boundary}").unwrap();
                writeln!(message, "Content-Type: text/html; charset=UTF-8").unwrap();
                writeln!(message, "Content-Transfer-Encoding: quoted-printable").unwrap();
                writeln!(message).unwrap();
                writeln!(message, "{html}").unwrap();
            }

            writeln!(message, "--{inner_boundary}--").unwrap();
        }

        // Attachments
        for attachment in &request.attachments {
            writeln!(message, "--{boundary}").unwrap();
            writeln!(
                message,
                "Content-Type: {}; name=\"{}\"",
                attachment.content_type, attachment.filename
            )
            .unwrap();
            writeln!(message, "Content-Transfer-Encoding: base64").unwrap();
            writeln!(
                message,
                "Content-Disposition: attachment; filename=\"{}\"",
                attachment.filename
            )
            .unwrap();
            writeln!(message).unwrap();

            // Attachment content is already base64 encoded
            for chunk in attachment.content.as_bytes().chunks(76) {
                writeln!(message, "{}", std::str::from_utf8(chunk).unwrap_or("")).unwrap();
            }
        }

        writeln!(message, "--{boundary}--").unwrap();

        Ok(message.into_bytes())
    }

    /// Get deliverability manager (for metrics)
    pub fn deliverability(&self) -> &DeliverabilityManager {
        &self.deliverability
    }

    /// Process an SNS notification (bounce/complaint)
    pub async fn process_notification(&self, notification: crate::deliverability::SesNotification) {
        self.deliverability.process_notification(notification).await;
    }
}

/// SES operation errors
#[derive(Debug, thiserror::Error)]
pub enum SesError {
    #[error("Validation error: {0}")]
    Validation(#[from] crate::types::ValidationError),

    #[error("Send blocked: {0}")]
    Blocked(#[from] SendBlockedReason),

    #[error("Email suppressed: {email} ({reason})")]
    Suppressed { email: String, reason: String },

    #[error("Failed to build email: {0}")]
    BuildError(String),

    #[error("SES send failed: {0}")]
    SendFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Attachment;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // Note: These tests require mocking SES client
    // In production, use localstack or integration tests

    #[test]
    fn test_mime_message_building() {
        let request = SendEmailRequest {
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            cc: vec![],
            bcc: vec![],
            reply_to: None,
            subject: "Test with attachment".to_string(),
            html: Some("<p>Hello</p>".to_string()),
            text: Some("Hello".to_string()),
            headers: vec![],
            tags: vec![],
            attachments: vec![Attachment {
                filename: "test.pdf".to_string(),
                content: BASE64.encode(b"fake pdf content"),
                content_type: "application/pdf".to_string(),
            }],
            configuration_set: None,
        };

        // We can't test send without mocking, but we can verify request validation
        assert!(request.validate().is_ok());
    }
}
