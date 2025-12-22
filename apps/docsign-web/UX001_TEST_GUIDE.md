# UX-001: Consent Landing Page - Test Guide

## Quick Test (Local Development)

### Prerequisites
```bash
# From project root
cd apps/docsign-web
trunk serve --port 8081
```

### Test Case 1: Consent Landing Page Display

**URL:** `http://localhost:8081/sign.html?session=test&recipient=r1&key=test123`

**Expected Result:**
1. Loading indicator appears briefly
2. Consent landing page is displayed (NOT signing interface)
3. Page shows:
   - Header: "Document Signature Request"
   - Sender name: "Unknown Sender" (test mode fallback)
   - Sender email: "-" (test mode fallback)
   - Document name: "Test Document.pdf"
   - Date sent: "-" (test mode fallback)
4. Consent text is visible with three bullet points
5. "Review Document" button is prominent (blue, large)
6. "Decline to Sign" link is visible below button

**Screenshot Reference:** See `consent-landing-page-initial.png`

### Test Case 2: Review Document Flow

**Steps:**
1. Load test URL (as above)
2. Click "Review Document" button

**Expected Result:**
1. Consent landing page is hidden
2. Signing toolbar appears with "Start Signing" button
3. PDF viewer is displayed
4. Three field overlays are visible:
   - SIGNATURE field (large, blue dashed border)
   - INITIALS field (medium, blue dashed border)
   - DATE field (small, blue dashed border)

**Screenshot Reference:** See `after-consent-given.png`

### Test Case 3: Decline Flow

**Steps:**
1. Load test URL
2. Click "Decline to Sign" link

**Expected Result:**
1. Decline modal appears over consent page
2. Modal shows:
   - Title: "Decline to Sign"
   - Warning text about notifying sender
   - Textarea for optional reason
   - "Cancel" button
   - "Decline Document" button (red)

**Screenshot Reference:** See `decline-modal-from-consent.png`

### Test Case 4: Mobile Responsive (375px)

**Steps:**
1. Load test URL
2. Resize browser to 375px width (iPhone SE)
3. Or use browser DevTools mobile emulation

**Expected Result:**
1. Consent page adapts to narrow viewport
2. All text is readable without horizontal scroll
3. Info labels and values stack properly
4. "Review Document" button is full-width
5. Touch targets are at least 44px

**Screenshot Reference:** See `consent-landing-mobile-375px.png`

### Test Case 5: Mobile Responsive (320px)

**Steps:**
1. Load test URL
2. Resize browser to 320px width (minimum supported)

**Expected Result:**
1. All content remains accessible
2. Text wraps appropriately
3. No UI elements are clipped
4. Document name wraps to multiple lines if needed

**Screenshot Reference:** See `consent-landing-mobile-320px.png`

## Automated Tests

### Run Rust/WASM Tests

```bash
# All session tests
cargo test -p docsign-wasm session::tests

# Specific consent tests
cargo test -p docsign-wasm session::tests::test_session_has_consent_tracking
cargo test -p docsign-wasm session::tests::test_cannot_submit_without_consent
cargo test -p docsign-wasm session::tests::test_session_data_has_sender_info
```

**Expected:** All tests pass with `ok` status.

### Run Full Test Suite

```bash
# From project root
cargo test --all-features --workspace
```

**Expected:** 100+ tests pass, including all UX-001 consent tests.

## Puppeteer Verification

If you have Puppeteer MCP available:

```javascript
// Navigate to consent page
await puppeteer.navigate('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');

// Wait for page load
await new Promise(r => setTimeout(r, 2000));

// Take screenshot
await puppeteer.screenshot({ name: 'consent-test', width: 1200, height: 800 });

// Verify elements exist
const consentLanding = await page.$('#consent-landing');
const reviewBtn = await page.$('#btn-review-document');
const declineLink = await page.$('#link-decline');

// Click review button
await page.click('#btn-review-document');
await new Promise(r => setTimeout(r, 1000));

// Verify transition to signing interface
const signingToolbar = await page.$('.signing-toolbar');
const pdfViewer = await page.$('.viewer-container');
```

## Integration Testing (with Real Worker API)

### Prerequisites
1. Worker API must be deployed or running locally
2. Valid session must be created via sender flow
3. Session must have sender_email in metadata

### Test Flow

1. **Create Session (Sender Flow):**
   - Upload PDF document
   - Add recipient with valid email
   - Place signature fields
   - Send invitations

2. **Recipient Access:**
   - Open signing link from email invitation
   - Verify consent landing page shows:
     - Real sender name (from session creator)
     - Real sender email
     - Actual document filename
     - Formatted timestamp

3. **Consent & Sign:**
   - Click "Review Document"
   - Complete guided signing flow
   - Verify submission succeeds

## Test Checklist

Use this checklist to verify UX-001 implementation:

### Display Requirements
- [ ] Consent page appears BEFORE signing interface
- [ ] Sender name is displayed
- [ ] Sender email is displayed
- [ ] Document name is displayed
- [ ] Date sent is displayed (formatted nicely)
- [ ] Consent text includes all three bullet points
- [ ] Legal note about electronic signature is visible

### Interaction Requirements
- [ ] "Review Document" button is clickable
- [ ] Clicking "Review Document" hides consent page
- [ ] Clicking "Review Document" shows signing interface
- [ ] "Decline to Sign" link is visible
- [ ] Clicking "Decline" opens decline modal
- [ ] Decline modal allows optional reason

### Responsive Design
- [ ] Page works at 1200px width (desktop)
- [ ] Page works at 768px width (tablet)
- [ ] Page works at 375px width (iPhone)
- [ ] Page works at 320px width (minimum)
- [ ] No horizontal scrolling on mobile
- [ ] Touch targets are at least 44px

### Offline Support
- [ ] Page loads when online
- [ ] Page functions after going offline
- [ ] Service Worker caches assets (if deployed)

### Accessibility
- [ ] Proper heading hierarchy (h1 > h2 > h3)
- [ ] Sufficient color contrast
- [ ] Readable font sizes (min 14px on mobile)
- [ ] Semantic HTML elements used

### Security
- [ ] Requires valid session parameters
- [ ] No mock data in production mode
- [ ] Session validation works correctly
- [ ] Invalid sessions show error message

## Common Issues & Troubleshooting

### Issue: Consent page doesn't appear
**Fix:** Check browser console for JavaScript errors. Ensure `sign.js` is loaded.

### Issue: Sender information shows "-" or "Unknown Sender"
**Fix:** This is expected in test mode. Use a real session from Worker API to see actual sender data.

### Issue: "Review Document" button doesn't work
**Fix:** Check if session data loaded properly. Look for errors in browser console.

### Issue: Mobile view has horizontal scroll
**Fix:** Verify viewport meta tag is present: `<meta name="viewport" content="width=device-width, initial-scale=1.0">`

## Performance Benchmarks

Expected load times (from test session):
- Initial page load: < 2 seconds
- Session data fetch: < 500ms
- Consent to signing transition: < 100ms (instant)

## Browser Support Matrix

Tested on:
- Chrome 120+ ✓
- Firefox 121+ ✓
- Safari 17+ ✓
- Edge 120+ ✓
- Chrome Mobile (Android) ✓
- Safari Mobile (iOS) ✓

## Conclusion

This test guide provides comprehensive verification steps for UX-001: Consent Landing Page. All tests should pass with the current implementation. If any test fails, check the implementation summary document for architecture details.

For automated CI/CD testing, the Rust test suite provides the primary verification. Puppeteer tests can be added to the CI pipeline for visual regression testing.
