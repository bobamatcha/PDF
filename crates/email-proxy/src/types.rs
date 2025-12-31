//! Email types and request/response structures
//!
//! Implements Resend-compatible API structures for easy migration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Email send request - Resend-compatible API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailRequest {
    /// Sender email address (must be verified in SES)
    pub from: String,

    /// Recipient email addresses
    pub to: Vec<String>,

    /// CC recipients (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,

    /// BCC recipients (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,

    /// Reply-to address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,

    /// Email subject
    pub subject: String,

    /// HTML body (optional, but recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,

    /// Plain text body (optional, but recommended for deliverability)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Custom headers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<EmailHeader>,

    /// Tags for tracking
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<EmailTag>,

    /// Attachments (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

    /// Configuration set name (for SES tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration_set: Option<String>,
}

impl SendEmailRequest {
    /// Create a simple email request
    pub fn simple(from: &str, to: &str, subject: &str, html: &str) -> Self {
        Self {
            from: from.to_string(),
            to: vec![to.to_string()],
            cc: vec![],
            bcc: vec![],
            reply_to: None,
            subject: subject.to_string(),
            html: Some(html.to_string()),
            text: None,
            headers: vec![],
            tags: vec![],
            attachments: vec![],
            configuration_set: None,
        }
    }

    /// Add a tag for tracking
    pub fn with_tag(mut self, name: &str, value: &str) -> Self {
        self.tags.push(EmailTag {
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Add plain text version (improves deliverability)
    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// Validate the request
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check sender
        if self.from.is_empty() {
            return Err(ValidationError::MissingField("from"));
        }
        if !is_valid_email(&self.from) {
            return Err(ValidationError::InvalidEmail(self.from.clone()));
        }

        // Check recipients
        if self.to.is_empty() {
            return Err(ValidationError::MissingField("to"));
        }
        for email in &self.to {
            if !is_valid_email(email) {
                return Err(ValidationError::InvalidEmail(email.clone()));
            }
        }

        // Check subject
        if self.subject.is_empty() {
            return Err(ValidationError::MissingField("subject"));
        }

        // Check content (need either html or text)
        if self.html.is_none() && self.text.is_none() {
            return Err(ValidationError::MissingContent);
        }

        Ok(())
    }
}

/// Custom email header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailHeader {
    pub name: String,
    pub value: String,
}

/// Email tag for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTag {
    pub name: String,
    pub value: String,
}

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Filename for the attachment
    pub filename: String,

    /// Base64-encoded content
    pub content: String,

    /// MIME type (e.g., "application/pdf")
    #[serde(default = "default_mime_type")]
    pub content_type: String,
}

fn default_mime_type() -> String {
    "application/octet-stream".to_string()
}

/// Response from send email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailResponse {
    /// Unique message ID from SES
    pub id: String,

    /// Timestamp when the email was queued
    pub queued_at: DateTime<Utc>,

    /// Status of the send operation
    pub status: EmailStatus,
}

/// Email delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmailStatus {
    /// Email queued for delivery
    Queued,
    /// Email sent to recipient's mail server
    Sent,
    /// Email delivered to inbox
    Delivered,
    /// Email bounced (hard or soft)
    Bounced,
    /// Recipient complained (marked as spam)
    Complained,
    /// Email rejected before sending
    Rejected,
}

/// Validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid email address: {0}")]
    InvalidEmail(String),

    #[error("Email must have either html or text content")]
    MissingContent,

    #[error("Attachment too large: {0} bytes (max {1})")]
    AttachmentTooLarge(usize, usize),
}

/// Check if email address is valid
fn is_valid_email(email: &str) -> bool {
    // Extract email from "Name <email@domain.com>" format
    let email = if email.contains('<') && email.contains('>') {
        let start = email.find('<').unwrap() + 1;
        let end = email.find('>').unwrap();
        &email[start..end]
    } else {
        email
    };

    email_address::EmailAddress::is_valid(email)
}

/// DocSign-specific email templates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmailTemplate {
    /// Request to sign a document
    SignatureRequest {
        signer_name: String,
        signer_email: String,
        sender_name: String,
        document_name: String,
        signing_url: String,
        message: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    },

    /// Reminder to sign
    SignatureReminder {
        signer_name: String,
        signer_email: String,
        sender_name: String,
        document_name: String,
        signing_url: String,
        days_remaining: u32,
    },

    /// Document signed notification to sender
    DocumentSigned {
        sender_name: String,
        sender_email: String,
        signer_name: String,
        document_name: String,
        view_url: String,
    },

    /// All signatures complete
    DocumentComplete {
        recipient_name: String,
        recipient_email: String,
        document_name: String,
        download_url: String,
    },

    /// Document was declined
    DocumentDeclined {
        sender_name: String,
        sender_email: String,
        signer_name: String,
        document_name: String,
        decline_reason: Option<String>,
    },
}

impl EmailTemplate {
    /// Convert template to SendEmailRequest
    pub fn to_request(&self, from: &str) -> SendEmailRequest {
        match self {
            EmailTemplate::SignatureRequest {
                signer_name,
                signer_email,
                sender_name,
                document_name,
                signing_url,
                message,
                expires_at,
            } => {
                let subject = format!("{} sent you \"{}\" to sign", sender_name, document_name);

                let expires_text = expires_at
                    .map(|d| format!("\n\nThis request expires on {}.", d.format("%B %d, %Y")))
                    .unwrap_or_default();

                let custom_message = message
                    .as_ref()
                    .map(|m| format!("\n\nMessage from {}:\n{}", sender_name, m))
                    .unwrap_or_default();

                let html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h1 style="color: #333; font-size: 24px;">You have a document to sign</h1>

  <p style="color: #666; font-size: 16px; line-height: 1.5;">
    Hi {signer_name},
  </p>

  <p style="color: #666; font-size: 16px; line-height: 1.5;">
    {sender_name} has sent you <strong>"{document_name}"</strong> to review and sign.
  </p>
  {custom_message_html}
  <div style="margin: 30px 0;">
    <a href="{signing_url}"
       style="display: inline-block; background-color: #2563eb; color: white; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600;">
      Review &amp; Sign Document
    </a>
  </div>
  {expires_html}
  <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">

  <p style="color: #999; font-size: 12px;">
    Powered by GetSignatures.org<br>
    This email was sent because {sender_name} requested your signature.
  </p>
</body>
</html>"#,
                    signer_name = signer_name,
                    sender_name = sender_name,
                    document_name = document_name,
                    signing_url = signing_url,
                    custom_message_html = if message.is_some() {
                        format!(
                            r#"<div style="background: #f9f9f9; padding: 15px; border-radius: 6px; margin: 20px 0;">
  <p style="color: #666; font-size: 14px; margin: 0;"><em>Message from {sender_name}:</em></p>
  <p style="color: #333; font-size: 14px; margin: 10px 0 0 0;">{msg}</p>
</div>"#,
                            sender_name = sender_name,
                            msg = message.as_deref().unwrap_or("")
                        )
                    } else {
                        String::new()
                    },
                    expires_html = if expires_at.is_some() {
                        format!(
                            r#"<p style="color: #dc2626; font-size: 14px;">This request expires on {}.</p>"#,
                            expires_at.unwrap().format("%B %d, %Y")
                        )
                    } else {
                        String::new()
                    }
                );

                let text = format!(
                    "Hi {signer_name},\n\n\
                    {sender_name} has sent you \"{document_name}\" to review and sign.\
                    {custom_message}\n\n\
                    Review and sign here: {signing_url}\
                    {expires_text}\n\n\
                    ---\n\
                    Powered by GetSignatures.org",
                );

                SendEmailRequest {
                    from: from.to_string(),
                    to: vec![signer_email.clone()],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: None,
                    subject,
                    html: Some(html),
                    text: Some(text),
                    headers: vec![
                        EmailHeader {
                            name: "X-Entity-Ref-ID".to_string(),
                            value: uuid::Uuid::new_v4().to_string(),
                        },
                        EmailHeader {
                            name: "List-Unsubscribe".to_string(),
                            value: format!(
                                "<mailto:unsubscribe@getsignatures.org?subject=unsubscribe>"
                            ),
                        },
                    ],
                    tags: vec![EmailTag {
                        name: "type".to_string(),
                        value: "signature_request".to_string(),
                    }],
                    attachments: vec![],
                    configuration_set: Some("docsign-transactional".to_string()),
                }
            }

            EmailTemplate::SignatureReminder {
                signer_name,
                signer_email,
                sender_name,
                document_name,
                signing_url,
                days_remaining,
            } => {
                let subject = format!("Reminder: \"{}\" awaits your signature", document_name);

                let urgency = if *days_remaining <= 1 {
                    "expires today"
                } else if *days_remaining <= 3 {
                    "expires soon"
                } else {
                    "is waiting"
                };

                let html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h1 style="color: #333; font-size: 24px;">Friendly Reminder</h1>

  <p style="color: #666; font-size: 16px; line-height: 1.5;">
    Hi {signer_name},
  </p>

  <p style="color: #666; font-size: 16px; line-height: 1.5;">
    <strong>"{document_name}"</strong> from {sender_name} {urgency} for your signature.
  </p>

  <div style="margin: 30px 0;">
    <a href="{signing_url}"
       style="display: inline-block; background-color: #2563eb; color: white; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600;">
      Sign Now
    </a>
  </div>

  <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">

  <p style="color: #999; font-size: 12px;">
    Powered by GetSignatures.org
  </p>
</body>
</html>"#
                );

                let text = format!(
                    "Hi {signer_name},\n\n\
                    \"{document_name}\" from {sender_name} {urgency} for your signature.\n\n\
                    Sign here: {signing_url}\n\n\
                    ---\n\
                    Powered by GetSignatures.org",
                );

                SendEmailRequest {
                    from: from.to_string(),
                    to: vec![signer_email.clone()],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: None,
                    subject,
                    html: Some(html),
                    text: Some(text),
                    headers: vec![EmailHeader {
                        name: "X-Entity-Ref-ID".to_string(),
                        value: uuid::Uuid::new_v4().to_string(),
                    }],
                    tags: vec![EmailTag {
                        name: "type".to_string(),
                        value: "signature_reminder".to_string(),
                    }],
                    attachments: vec![],
                    configuration_set: Some("docsign-transactional".to_string()),
                }
            }

            EmailTemplate::DocumentSigned {
                sender_name: _, // Not used in this template
                sender_email,
                signer_name,
                document_name,
                view_url,
            } => {
                let subject = format!("{} signed \"{}\"", signer_name, document_name);

                let html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <div style="text-align: center; margin-bottom: 30px;">
    <div style="display: inline-block; background: #dcfce7; border-radius: 50%; padding: 15px;">
      <span style="font-size: 32px;">✓</span>
    </div>
  </div>

  <h1 style="color: #333; font-size: 24px; text-align: center;">Document Signed!</h1>

  <p style="color: #666; font-size: 16px; line-height: 1.5; text-align: center;">
    {signer_name} has signed <strong>"{document_name}"</strong>.
  </p>

  <div style="margin: 30px 0; text-align: center;">
    <a href="{view_url}"
       style="display: inline-block; background-color: #2563eb; color: white; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600;">
      View Document
    </a>
  </div>

  <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">

  <p style="color: #999; font-size: 12px; text-align: center;">
    Powered by GetSignatures.org
  </p>
</body>
</html>"#
                );

                let text = format!(
                    "{signer_name} has signed \"{document_name}\".\n\n\
                    View document: {view_url}\n\n\
                    ---\n\
                    Powered by GetSignatures.org",
                );

                SendEmailRequest {
                    from: from.to_string(),
                    to: vec![sender_email.clone()],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: None,
                    subject,
                    html: Some(html),
                    text: Some(text),
                    headers: vec![],
                    tags: vec![EmailTag {
                        name: "type".to_string(),
                        value: "document_signed".to_string(),
                    }],
                    attachments: vec![],
                    configuration_set: Some("docsign-transactional".to_string()),
                }
            }

            EmailTemplate::DocumentComplete {
                recipient_name,
                recipient_email,
                document_name,
                download_url,
            } => {
                let subject = format!("Completed: \"{}\"", document_name);

                let html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <div style="text-align: center; margin-bottom: 30px;">
    <div style="display: inline-block; background: #dcfce7; border-radius: 50%; padding: 15px;">
      <span style="font-size: 32px;">✓</span>
    </div>
  </div>

  <h1 style="color: #333; font-size: 24px; text-align: center;">All Signatures Complete!</h1>

  <p style="color: #666; font-size: 16px; line-height: 1.5; text-align: center;">
    Hi {recipient_name}, all parties have signed <strong>"{document_name}"</strong>.
  </p>

  <div style="margin: 30px 0; text-align: center;">
    <a href="{download_url}"
       style="display: inline-block; background-color: #16a34a; color: white; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600;">
      Download Signed Document
    </a>
  </div>

  <p style="color: #666; font-size: 14px; text-align: center;">
    The document includes a tamper-evident audit trail with all signatures.
  </p>

  <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">

  <p style="color: #999; font-size: 12px; text-align: center;">
    Powered by GetSignatures.org
  </p>
</body>
</html>"#
                );

                let text = format!(
                    "Hi {recipient_name},\n\n\
                    All parties have signed \"{document_name}\".\n\n\
                    Download the signed document: {download_url}\n\n\
                    The document includes a tamper-evident audit trail with all signatures.\n\n\
                    ---\n\
                    Powered by GetSignatures.org",
                );

                SendEmailRequest {
                    from: from.to_string(),
                    to: vec![recipient_email.clone()],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: None,
                    subject,
                    html: Some(html),
                    text: Some(text),
                    headers: vec![],
                    tags: vec![EmailTag {
                        name: "type".to_string(),
                        value: "document_complete".to_string(),
                    }],
                    attachments: vec![],
                    configuration_set: Some("docsign-transactional".to_string()),
                }
            }

            EmailTemplate::DocumentDeclined {
                sender_name: _, // Not used in this template
                sender_email,
                signer_name,
                document_name,
                decline_reason,
            } => {
                let subject = format!("{} declined \"{}\"", signer_name, document_name);

                let reason_html = decline_reason
                    .as_ref()
                    .map(|r| {
                        format!(
                            r#"<div style="background: #fef2f2; padding: 15px; border-radius: 6px; margin: 20px 0;">
  <p style="color: #991b1b; font-size: 14px; margin: 0;"><strong>Reason:</strong></p>
  <p style="color: #333; font-size: 14px; margin: 10px 0 0 0;">{}</p>
</div>"#,
                            r
                        )
                    })
                    .unwrap_or_default();

                let reason_text = decline_reason
                    .as_ref()
                    .map(|r| format!("\n\nReason: {}", r))
                    .unwrap_or_default();

                let html = format!(
                    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <div style="text-align: center; margin-bottom: 30px;">
    <div style="display: inline-block; background: #fef2f2; border-radius: 50%; padding: 15px;">
      <span style="font-size: 32px;">✕</span>
    </div>
  </div>

  <h1 style="color: #333; font-size: 24px; text-align: center;">Document Declined</h1>

  <p style="color: #666; font-size: 16px; line-height: 1.5; text-align: center;">
    {signer_name} has declined to sign <strong>"{document_name}"</strong>.
  </p>

  {reason_html}

  <p style="color: #666; font-size: 14px; text-align: center;">
    You may contact {signer_name} to discuss and resend if needed.
  </p>

  <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">

  <p style="color: #999; font-size: 12px; text-align: center;">
    Powered by GetSignatures.org
  </p>
</body>
</html>"#
                );

                let text = format!(
                    "{signer_name} has declined to sign \"{document_name}\".{reason_text}\n\n\
                    You may contact {signer_name} to discuss and resend if needed.\n\n\
                    ---\n\
                    Powered by GetSignatures.org",
                );

                SendEmailRequest {
                    from: from.to_string(),
                    to: vec![sender_email.clone()],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: None,
                    subject,
                    html: Some(html),
                    text: Some(text),
                    headers: vec![],
                    tags: vec![EmailTag {
                        name: "type".to_string(),
                        value: "document_declined".to_string(),
                    }],
                    attachments: vec![],
                    configuration_set: Some("docsign-transactional".to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_email_request() {
        let req = SendEmailRequest::simple(
            "noreply@getsignatures.org",
            "user@example.com",
            "Test Subject",
            "<p>Hello</p>",
        );

        assert_eq!(req.from, "noreply@getsignatures.org");
        assert_eq!(req.to, vec!["user@example.com"]);
        assert_eq!(req.subject, "Test Subject");
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_email_with_display_name() {
        let req = SendEmailRequest::simple(
            "GetSignatures <noreply@getsignatures.org>",
            "John Doe <john@example.com>",
            "Test",
            "<p>Hello</p>",
        );

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_missing_recipient() {
        let mut req =
            SendEmailRequest::simple("from@example.com", "to@example.com", "Test", "<p>Hello</p>");
        req.to.clear();

        assert!(matches!(
            req.validate(),
            Err(ValidationError::MissingField("to"))
        ));
    }

    #[test]
    fn test_invalid_email() {
        let req = SendEmailRequest::simple("invalid-email", "to@example.com", "Test", "<p>Hi</p>");

        assert!(matches!(
            req.validate(),
            Err(ValidationError::InvalidEmail(_))
        ));
    }

    #[test]
    fn test_signature_request_template() {
        let template = EmailTemplate::SignatureRequest {
            signer_name: "John Doe".to_string(),
            signer_email: "john@example.com".to_string(),
            sender_name: "Jane Smith".to_string(),
            document_name: "Contract.pdf".to_string(),
            signing_url: "https://sign.getsignatures.org/abc123".to_string(),
            message: Some("Please sign by Friday".to_string()),
            expires_at: None,
        };

        let req = template.to_request("noreply@getsignatures.org");
        assert!(req.validate().is_ok());
        assert!(req.subject.contains("Jane Smith"));
        assert!(req.subject.contains("Contract.pdf"));
        assert!(req
            .html
            .as_ref()
            .unwrap()
            .contains("Review &amp; Sign Document"));
    }
}
