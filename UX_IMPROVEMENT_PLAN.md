# UX Improvement Plan: docsign-web

This document outlines UX improvements needed to bring docsign-web (getsignatures.org) to parity with industry-standard e-signature flows like DocuSign.

**Target:** Each feature is designed as an independent, parallelizable task for Claude Code agents.

**Methodology:** All features MUST follow the test-first development flow defined in `CLAUDE.md`.

---

## Feature Index

| ID | Feature | Priority | Parallelizable | Dependencies |
|----|---------|----------|----------------|--------------|
| UX-001 | Consent Landing Page | P0 | Yes | None |
| UX-002 | Accept/Decline Flow | P0 | Yes | None |
| UX-003 | Email Identity Verification | P1 | Yes | None |
| UX-004 | Session Expiry & Resend | P1 | Yes | UX-002 |
| UX-005 | Mobile-Optimized Signing | P2 | Yes | None |
| UX-006 | Sender Notification on Sign | P1 | Yes | None |

**Parallel Execution Groups:**
- **Group A** (can run simultaneously): UX-001, UX-002, UX-003, UX-005, UX-006
- **Group B** (after UX-002): UX-004

---

## UX-001: Consent Landing Page

### Description
Recipients currently land directly on the signing page with no context. Add a landing page that explains what they're about to sign, who sent it, and obtains electronic signature consent.

### Current Behavior
- `sign.html` loads directly into the PDF viewer
- No sender information displayed prominently
- No consent acknowledgment required

### Desired Behavior
- Show a landing page BEFORE the signing interface
- Display: sender name, sender email, document name, date sent
- Show legal consent text: "By clicking 'Review Document', you agree to use electronic signatures"
- "Review Document" button proceeds to signing interface
- "Decline" link visible (links to decline flow)

### Acceptance Criteria
```
[ ] Landing page displays sender name from session data
[ ] Landing page displays sender email from session data
[ ] Landing page displays document filename
[ ] Landing page displays date document was sent
[ ] Consent text is visible and matches legal requirements
[ ] "Review Document" button is prominent and clickable
[ ] Clicking "Review Document" transitions to signing interface
[ ] "Decline" option is visible
[ ] Landing page is responsive (mobile-friendly)
[ ] Landing page works offline (after initial load)
```

### Files to Modify
- `apps/docsign-web/www/sign.html` - Add landing page HTML structure
- `apps/docsign-web/www/sign.js` - Add landing page state management
- `apps/docsign-web/worker/src/lib.rs` - Ensure session endpoint returns sender info

### Test Strategy

**Rust/WASM Tests (write first, must fail):**
```rust
// In apps/docsign-web/wasm/src/session/mod.rs or new test file
#[cfg(test)]
mod consent_landing_tests {
    #[test]
    fn test_session_contains_sender_info() {
        // Session data must include sender_name, sender_email, sent_at
    }

    #[test]
    fn test_consent_not_given_blocks_signing() {
        // Signing should fail if consent flag is false
    }
}
```

**Puppeteer Verification:**
1. Navigate to `sign.html?session=X&recipient=Y&key=Z`
2. Verify landing page appears (not PDF viewer)
3. Verify sender info displayed correctly
4. Click "Review Document"
5. Verify PDF viewer now visible

---

## UX-002: Accept/Decline Flow

### Description
Add explicit accept/decline step before signing, with decline reason capture.

### Current Behavior
- No explicit accept step
- Decline button exists but flow is unclear
- No reason capture on decline

### Desired Behavior
- After consent landing, show "Accept" or "Decline" choice
- "Accept" proceeds to signing
- "Decline" shows modal asking for reason (optional text field)
- Declined documents notify sender
- Session marked as declined in KV store

### Acceptance Criteria
```
[ ] Accept/Decline buttons displayed after consent
[ ] Clicking "Accept" proceeds to signing interface
[ ] Clicking "Decline" opens decline modal
[ ] Decline modal has optional reason text field
[ ] Decline modal has "Confirm Decline" and "Cancel" buttons
[ ] Confirming decline calls worker endpoint
[ ] Worker marks session as declined
[ ] Worker sends notification email to sender
[ ] Declined sessions show "This document was declined" on revisit
[ ] Audit log records decline event with reason
```

### Files to Modify
- `apps/docsign-web/www/sign.html` - Add decline modal HTML
- `apps/docsign-web/www/sign.js` - Add decline flow logic
- `apps/docsign-web/worker/src/lib.rs` - Add `PUT /session/{id}/decline` endpoint
- `apps/docsign-web/wasm/src/pdf/audit.rs` - Ensure `Decline` event captures reason

### Test Strategy

**Rust/WASM Tests (write first, must fail):**
```rust
#[cfg(test)]
mod decline_flow_tests {
    #[test]
    fn test_decline_event_includes_reason() {
        // AuditEvent::Decline must have reason field
    }

    #[test]
    fn test_declined_session_blocks_signing() {
        // Attempting to sign a declined session should error
    }
}

// Worker tests
#[cfg(test)]
mod worker_decline_tests {
    #[tokio::test]
    async fn test_decline_endpoint_updates_session() {
        // PUT /session/{id}/decline should set status = "declined"
    }

    #[tokio::test]
    async fn test_decline_sends_notification_email() {
        // Decline should trigger email to sender
    }
}
```

**Puppeteer Verification:**
1. Navigate to signing URL
2. Pass consent page
3. Click "Decline"
4. Verify modal appears
5. Enter reason, confirm
6. Verify confirmation message
7. Revisit URL, verify shows "declined" state

---

## UX-003: Email Identity Verification

### Description
Add lightweight identity verification to ensure the person clicking the link is the intended recipient.

### Current Behavior
- Anyone with the link can sign
- No verification that signer received email at their address

### Desired Behavior
- After clicking link, prompt for last 4 characters of email (e.g., "Verify: ***@gm]")
- 3 attempts allowed before lockout
- Lockout duration: 15 minutes
- Successful verification stored in session/localStorage
- Optional: Send 6-digit code to email for high-security documents

### Acceptance Criteria
```
[ ] Verification prompt displayed before consent page
[ ] Prompt shows masked email hint (e.g., "j***@gmail.com")
[ ] Input field for last 4 characters of email
[ ] Correct input proceeds to consent page
[ ] Incorrect input shows error, decrements attempts
[ ] 3 failed attempts triggers 15-minute lockout
[ ] Lockout persists across page refreshes (localStorage)
[ ] Lockout shows countdown timer
[ ] Successful verification stored to skip on refresh
[ ] Worker tracks verification attempts per session/recipient
```

### Files to Modify
- `apps/docsign-web/www/sign.html` - Add verification UI
- `apps/docsign-web/www/sign.js` - Add verification logic
- `apps/docsign-web/worker/src/lib.rs` - Add verification attempt tracking
- New: `apps/docsign-web/www/verification.js` - Verification module (optional)

### Test Strategy

**Rust/WASM Tests (write first, must fail):**
```rust
#[cfg(test)]
mod verification_tests {
    #[test]
    fn test_mask_email_correctly() {
        // "john@gmail.com" -> "j***@gmail.com"
        // "ab@x.co" -> "a***@x.co"
    }

    #[test]
    fn test_verify_email_suffix() {
        // "john@gmail.com" with input "l.com" -> true
        // "john@gmail.com" with input "mail" -> false
    }
}
```

**Worker Tests:**
```rust
#[cfg(test)]
mod verification_worker_tests {
    #[tokio::test]
    async fn test_verification_attempt_tracking() {
        // Track attempts in KV, return remaining attempts
    }

    #[tokio::test]
    async fn test_lockout_after_3_failures() {
        // After 3 failures, return lockout with expiry time
    }
}
```

**Puppeteer Verification:**
1. Navigate to signing URL
2. Verify email verification prompt appears
3. Enter wrong suffix 3 times
4. Verify lockout message with timer
5. Wait or reset, enter correct suffix
6. Verify proceeds to consent page

---

## UX-004: Session Expiry & Resend

### Description
Improve session expiry handling with clear messaging and resend capability.

### Current Behavior
- Sessions expire after 7 days silently
- No way for sender to resend
- No notification before expiry

### Desired Behavior
- Show clear "This link has expired" page for expired sessions
- Include sender contact info on expiry page
- Add "Request New Link" button that notifies sender
- Sender dashboard shows pending/expired documents
- Sender can resend invitations

### Acceptance Criteria
```
[ ] Expired sessions show dedicated expiry page (not error)
[ ] Expiry page shows document name and sender info
[ ] "Request New Link" button visible on expiry page
[ ] Clicking request sends notification to sender
[ ] Sender receives email with link to resend
[ ] Worker endpoint for resend generates new session
[ ] Old session links redirect to "expired" page
[ ] 24-hour reminder email sent before expiry (optional)
[ ] Session expiry extended by 7 days on resend
```

### Files to Modify
- `apps/docsign-web/www/sign.html` - Add expiry page template
- `apps/docsign-web/www/sign.js` - Handle expired session state
- `apps/docsign-web/worker/src/lib.rs` - Add resend endpoint, expiry detection
- `apps/docsign-web/www/index.html` - Add pending documents view (sender side)

### Test Strategy

**Worker Tests (write first, must fail):**
```rust
#[cfg(test)]
mod session_expiry_tests {
    #[tokio::test]
    async fn test_expired_session_returns_expired_status() {
        // GET /session/{id} with expired TTL returns { status: "expired" }
    }

    #[tokio::test]
    async fn test_resend_creates_new_session() {
        // POST /session/{id}/resend creates new session, invalidates old
    }

    #[tokio::test]
    async fn test_request_link_notifies_sender() {
        // POST /session/{id}/request-link sends email to sender
    }
}
```

**Puppeteer Verification:**
1. Create session with short TTL (test mode)
2. Wait for expiry
3. Navigate to signing URL
4. Verify expiry page displayed
5. Click "Request New Link"
6. Verify confirmation message
7. Check sender received notification

---

## UX-005: Mobile-Optimized Signing

### Description
Ensure the signing experience is fully optimized for mobile devices.

### Current Behavior
- Guided flow exists but may not be fully mobile-optimized
- Signature pad may be too small on mobile
- Field navigation may be difficult on small screens

### Desired Behavior
- Full-screen signature capture on mobile
- Large, touch-friendly buttons
- Swipe gestures for field navigation
- Pinch-to-zoom on PDF
- Portrait and landscape support
- Bottom sheet for signature modal (not centered modal)

### Acceptance Criteria
```
[ ] Signature modal is full-screen on mobile (< 768px)
[ ] Signature pad fills available width with proper aspect ratio
[ ] Touch targets are minimum 44x44px
[ ] Swipe left/right navigates between fields
[ ] Pinch-to-zoom works on PDF viewer
[ ] UI adapts to portrait and landscape
[ ] Bottom sheet pattern for signature modal on mobile
[ ] "Next" button sticky at bottom of screen
[ ] Progress indicator visible without scrolling
[ ] No horizontal scrolling on any mobile view
```

### Files to Modify
- `apps/docsign-web/www/sign.html` - Add mobile-specific CSS, meta viewport
- `apps/docsign-web/www/sign.js` - Add touch gesture handlers
- `apps/docsign-web/www/guided-flow.js` - Add swipe navigation
- `apps/docsign-web/www/signature-pad.js` - Optimize for mobile dimensions

### Test Strategy

**Puppeteer Tests (viewport simulation):**
```javascript
// Test at mobile viewport
await page.setViewport({ width: 375, height: 667 }); // iPhone SE

// Verify signature modal is full-screen
const modal = await page.$('.signature-modal');
const box = await modal.boundingBox();
expect(box.width).toBeCloseTo(375, 5);

// Verify touch targets
const buttons = await page.$$('button');
for (const btn of buttons) {
  const box = await btn.boundingBox();
  expect(box.width).toBeGreaterThanOrEqual(44);
  expect(box.height).toBeGreaterThanOrEqual(44);
}
```

**Puppeteer Verification:**
1. Set viewport to 375x667 (iPhone SE)
2. Navigate to signing URL
3. Complete consent flow
4. Open signature modal
5. Verify full-screen behavior
6. Draw signature with touch simulation
7. Navigate to next field
8. Complete signing
9. Repeat at 414x896 (iPhone 11)
10. Test landscape orientation

---

## UX-006: Sender Notification on Sign

### Description
Notify sender when recipient completes signing.

### Current Behavior
- Sender has no visibility into signing status
- No email notification on completion
- No dashboard to check status

### Desired Behavior
- Email sender when each recipient signs
- Email sender when all recipients complete
- Include download link for signed document
- Show signing timestamp in notification

### Acceptance Criteria
```
[ ] Worker sends email to sender when recipient signs
[ ] Email includes: recipient name, document name, timestamp
[ ] Email includes secure link to download signed PDF
[ ] When all recipients sign, send "completed" summary email
[ ] Completed email includes combined signed PDF
[ ] Download links expire after 30 days
[ ] Audit log updated with notification events
```

### Files to Modify
- `apps/docsign-web/worker/src/lib.rs` - Add notification logic to signed endpoint
- New email template for sign completion notification

### Test Strategy

**Worker Tests (write first, must fail):**
```rust
#[cfg(test)]
mod notification_tests {
    #[tokio::test]
    async fn test_sign_triggers_sender_notification() {
        // PUT /session/{id}/signed should send email to sender
    }

    #[tokio::test]
    async fn test_all_signed_sends_completion_email() {
        // When last recipient signs, send completion summary
    }

    #[tokio::test]
    async fn test_notification_includes_download_link() {
        // Email body must contain valid download URL
    }

    #[tokio::test]
    async fn test_download_link_expires_after_30_days() {
        // Download link with timestamp, rejected after expiry
    }
}
```

**Puppeteer Verification:**
1. Create document with 2 recipients
2. Sign as recipient 1
3. Verify sender receives individual notification
4. Sign as recipient 2
5. Verify sender receives completion email
6. Click download link in email
7. Verify signed PDF downloads

---

## Implementation Notes

### For Each Feature, Follow This Order:

1. **Write failing tests** in Rust (`cargo test` must fail with expected failures)
2. **Confirm tests fail** with `cargo test --all-features --workspace`
3. **Implement the feature** (minimal changes only)
4. **Confirm tests pass** with `cargo test --all-features --workspace`
5. **Verify with Puppeteer** using the verification steps above
6. **If Puppeteer shows bugs**, rewrite tests and repeat from step 2

### Shared Test Utilities

If tests require mocking the Cloudflare Worker or session data, create shared utilities in:
```
apps/docsign-web/wasm/src/test_utils.rs  (for WASM tests)
apps/docsign-web/worker/src/test_utils.rs  (for Worker tests)
```

### Environment Setup for Testing

```bash
# Run all tests
cargo test --all-features --workspace

# Run docsign-web specific tests
cargo test -p docsign-wasm
cargo test -p docsign-worker

# Build and serve for Puppeteer testing
cd apps/docsign-web
trunk serve --port 8081
```

### Puppeteer Test Base URL

```javascript
const BASE_URL = 'http://localhost:8081';
const SIGN_URL = `${BASE_URL}/sign.html`;
```

---

## Agent Execution Commands

To implement these features in parallel, use:

```
Feature UX-001: "Implement consent landing page for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-001 for requirements and acceptance criteria."

Feature UX-002: "Implement accept/decline flow for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-002 for requirements and acceptance criteria."

Feature UX-003: "Implement email identity verification for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-003 for requirements and acceptance criteria."

Feature UX-005: "Implement mobile-optimized signing for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-005 for requirements and acceptance criteria."

Feature UX-006: "Implement sender notification on sign for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-006 for requirements and acceptance criteria."
```

After UX-002 completes:
```
Feature UX-004: "Implement session expiry and resend for docsign-web following test-first flow in CLAUDE.md. See UX_IMPROVEMENT_PLAN.md section UX-004 for requirements and acceptance criteria."
```
