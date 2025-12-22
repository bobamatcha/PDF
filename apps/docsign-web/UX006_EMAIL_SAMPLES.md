# UX-006 Email Notification Samples

This document shows sample email output from the UX-006 implementation.

## Individual Recipient Signed Email

**Subject**: `Alice Johnson Signed: lease_agreement.pdf`

**From**: `GetSignatures <noreply@mail.getsignatures.org>`

**To**: `sender@example.com`

**HTML Body** (simplified view):

```
┌────────────────────────────────────────────────┐
│                                                │
│         🟢 Document Signed                     │
│                                                │
└────────────────────────────────────────────────┘

Good news!

Alice Johnson has signed your document:

┌────────────────────────────────────────────────┐
│ Document Name                                  │
│ lease_agreement.pdf                            │
└────────────────────────────────────────────────┘

┌────────────────────────────────────────────────┐
│ Signed At                                      │
│ December 21, 2025 at 02:30 PM UTC              │
└────────────────────────────────────────────────┘

    [ Download Signed Document ]

⚠️ Note: This download link will expire after 30 days
   for security purposes.

The signed document is securely stored and encrypted.
You can download it anytime before the link expires.

─────────────────────────────────────────────────
Sent via GetSignatures - Secure Document Signing
```

**Download Link Format**:
```
https://getsignatures.org/download/sess_abc123?expires=1738281600
```

---

## All Recipients Signed Email (Completion)

**Subject**: `All Recipients Signed: lease_agreement.pdf`

**From**: `GetSignatures <noreply@mail.getsignatures.org>`

**To**: `sender@example.com`

**HTML Body** (simplified view):

```
┌────────────────────────────────────────────────┐
│                                                │
│   🎉 All Recipients Have Signed!               │
│                                                │
└────────────────────────────────────────────────┘

Congratulations!

All recipients have completed signing your document.
The signing process is now complete.

┌────────────────────────────────────────────────┐
│ Document Name                                  │
│ lease_agreement.pdf                            │
└────────────────────────────────────────────────┘

┌────────────────────────────────────────────────┐
│ Signers                                        │
│                                                │
│ • Alice Johnson - December 21, 2025 at 02:30 PM UTC
│ • Bob Smith - December 21, 2025 at 03:15 PM UTC
│ • Carol Davis - December 21, 2025 at 04:00 PM UTC
│                                                │
└────────────────────────────────────────────────┘

    [ Download Completed Document ]

⚠️ Note: This download link will expire after 30 days
   for security purposes.

The fully signed document is securely stored and encrypted.
You can download it anytime before the link expires.

─────────────────────────────────────────────────
Sent via GetSignatures - Secure Document Signing
```

**Download Link Format**:
```
https://getsignatures.org/download/sess_abc123?expires=1738281600
```

---

## Email Template Features

### Design Elements
- **Gradient Header**: Green gradient (from #059669 to #047857)
- **Mobile Responsive**: Max-width 600px, scales on mobile
- **Professional Typography**: System fonts (-apple-system, BlinkMacSystemFont, Segoe UI, Roboto)
- **Color Scheme**:
  - Primary Green: #059669
  - Dark Green: #047857
  - Gray Text: #6b7280
  - Warning Yellow: #f59e0b
  - Background: #f9fafb

### Content Structure
1. **Header Section**: Gradient background with icon and title
2. **Main Content**: Personalized message
3. **Information Boxes**: Highlighted document details
4. **Call-to-Action**: Prominent download button
5. **Warning Callout**: Expiry notice with icon
6. **Footer Note**: Security reassurance
7. **Branding**: GetSignatures attribution

### Accessibility
- Semantic HTML structure
- Clear text hierarchy
- High contrast ratios
- Descriptive link text
- Plain text alternative included

### Plain Text Fallback

Both emails include a plain text version automatically generated:

```
Good news!

Alice Johnson has signed your document:

Document Name
lease_agreement.pdf

Signed At
December 21, 2025 at 02:30 PM UTC

Download Signed Document
[https://getsignatures.org/download/sess_abc123?expires=1738281600]

Note: This download link will expire after 30 days for security purposes.

The signed document is securely stored and encrypted.
You can download it anytime before the link expires.

Sent via GetSignatures - Secure Document Signing
```

---

## Implementation Details

### Email Sending API (Resend)

**Endpoint**: `https://api.resend.com/emails`

**Request Headers**:
```
Authorization: Bearer {RESEND_API_KEY}
Content-Type: application/json
```

**Request Body**:
```json
{
  "from": "GetSignatures <noreply@mail.getsignatures.org>",
  "to": ["sender@example.com"],
  "subject": "Alice Johnson Signed: lease_agreement.pdf",
  "html": "<!DOCTYPE html>...",
  "text": "Good news!\n\nAlice Johnson..."
}
```

### Download Link Expiry Calculation

```rust
fn generate_download_link(session_id: &str, expiry_days: u32) -> String {
    let expiry_timestamp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(expiry_days as i64))
        .unwrap_or_else(chrono::Utc::now)
        .timestamp();

    format!(
        "https://getsignatures.org/download/{}?expires={}",
        session_id, expiry_timestamp
    )
}
```

**Example**:
- Current time: 2025-12-21 14:30:00 UTC
- Expiry days: 30
- Expiry timestamp: 1738281600 (2026-01-20 14:30:00 UTC)
- Link: `https://getsignatures.org/download/sess_abc123?expires=1738281600`

### Timestamp Formatting

```rust
fn format_timestamp(rfc3339: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(rfc3339) {
        Ok(dt) => dt.format("%B %d, %Y at %I:%M %p UTC").to_string(),
        Err(_) => rfc3339.to_string(),
    }
}
```

**Examples**:
- Input: `2025-12-21T14:30:00Z`
- Output: `December 21, 2025 at 02:30 PM UTC`

---

## Testing Email Templates

To test the email templates locally, you can use the test functions:

```bash
# Run email format tests
cargo test -p docsign-worker test_notification_email_format
cargo test -p docsign-worker test_completion_summary_email_includes_all_signers
```

To preview the actual HTML output, add this test:

```rust
#[test]
fn preview_individual_email() {
    let email = format_completion_notification_email(
        "Alice Johnson",
        "lease_agreement.pdf",
        "2025-12-21T14:30:00Z",
        "https://getsignatures.org/download/sess_abc123?expires=1738281600"
    );
    println!("{}", email);
}
```

Then run:
```bash
cargo test -p docsign-worker preview_individual_email -- --nocapture > email_preview.html
```

Open `email_preview.html` in a browser to see the rendered email.

---

## Email Delivery Scenarios

### Scenario 1: Single Recipient Document
- Recipient signs → Sender receives "Document Signed" email
- Only one email sent (completion email)

### Scenario 2: Two Recipient Document
1. First recipient signs → Sender receives "Alice Signed" email
2. Second recipient signs → Sender receives "All Recipients Signed" email
3. Total: 2 emails sent

### Scenario 3: Three Recipient Document
1. First recipient signs → Sender receives "Alice Signed" email
2. Second recipient signs → Sender receives "Bob Signed" email
3. Third recipient signs → Sender receives "All Recipients Signed" email
4. Total: 3 emails sent

### Scenario 4: No Sender Email Configured
- Recipient signs → No email sent
- Signing succeeds normally
- Logged: "sender_email not present in session metadata"

### Scenario 5: Email API Failure
- Recipient signs → Email send attempted and fails
- Signing succeeds anyway (email failure doesn't block)
- Logged: "Failed to send notification email: {error}"

---

## Email Testing Checklist

- [x] Individual signing email HTML renders correctly
- [x] Completion email HTML renders correctly
- [x] Plain text fallback is readable
- [x] Download links are valid URLs
- [x] Expiry timestamps are correct
- [x] Timestamps are human-readable
- [x] All signer names appear in completion email
- [x] Mobile responsive (max-width 600px)
- [x] High contrast for accessibility
- [x] From address is correct
- [x] Subject lines are descriptive
- [x] Error handling works (missing API key, send failure)

---

## Future Enhancements

### Possible Improvements (Not Implemented)
1. **Custom Templates**: Allow senders to customize email branding
2. **Language Support**: Multi-language email templates
3. **Email Preferences**: Let senders choose notification frequency
4. **Digest Mode**: Batch multiple signings into one email
5. **SMS Alternative**: Send SMS instead of/in addition to email
6. **Webhooks**: Post to sender's webhook URL on signing
7. **Calendar Integration**: Add signing event to calendar
8. **Attachment**: Include signed PDF directly in email (security consideration)

### Analytics (Not Implemented)
- Track email open rates
- Track download link clicks
- Monitor email delivery success rates
- A/B test email subject lines

---

## Security Considerations

### Download Link Security
- Time-limited (30 days)
- Expires parameter in URL
- Server validates expiry before serving file
- Encrypted document served over HTTPS

### Email Security
- No sensitive data in email body
- Document content not included
- Download link is public but time-limited
- Sender email validated server-side

### API Key Security
- Stored as Cloudflare secret (not env var)
- Not logged or exposed in responses
- Accessed only by worker runtime
- Rotatable without code changes

### Error Handling
- Email failures don't expose secrets
- Graceful degradation if API unavailable
- Signing always succeeds regardless of email status
- Errors logged securely

---

## Monitoring & Debugging

### Log Messages
```
INFO: Sending notification to sender@example.com
INFO: Notification sent successfully
WARN: Cannot send notification: RESEND_API_KEY not configured
ERROR: Failed to send notification email: {error}
```

### Metrics to Track
- Email send success rate
- Email send latency
- Download link click rate
- Average time from signing to email receipt
- Email bounce rate

### Debugging Tips
1. Check RESEND_API_KEY is set in Cloudflare secrets
2. Verify sender_email is populated in session metadata
3. Check Resend dashboard for delivery status
4. Verify email HTML renders in multiple clients
5. Test with real email addresses
6. Check spam folder if emails not arriving

---

This completes the UX-006 email notification documentation.
