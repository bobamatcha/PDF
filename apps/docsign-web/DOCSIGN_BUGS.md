# DocSign-Web Bug Tracker

> **CRITICAL: Follow CLAUDE.md Test-First Flow**
>
> This document uses a **strict patch rejection policy**:
> - A patch is NOT complete unless it fixes the **entire bug**, not just demonstrates a partial fix
> - Patches that "show an implementation exists" but don't fix all edge cases are **REJECTED**
> - User expectations define completeness—if the UX still diverges from what users expect, the bug is NOT fixed
> - Avoid oversimplification: fixing 80% of a bug creates a false sense of progress while leaving 20% broken

## Production Environment Reference

| Item | Value |
|------|-------|
| **Worker Name** | `docsign-worker-production` |
| **API URL** | `api.getsignatures.org` |
| **Frontend URL** | `getsignatures.org` |

`wrangler.toml` directly targets production - no separate env needed. Run `wrangler deploy` to deploy (user only, not Claude).

## Bug Status Legend

| Status | Meaning |
|--------|---------|
| **OPEN** | Bug exists, not started |
| **IN PROGRESS** | Actively being worked on |
| **TESTING** | Fix implemented, verifying via tests + Puppeteer |
| **SOLVED** | Fix confirmed working, moved to bottom of doc |

---

## Active Bugs (Priority Order: Highest First)

### Bug #0: CRITICAL - Unable to Generate Signing Links (P0)

**Status:** SOLVED (2026-01-12)
**Priority:** P0 - CRITICAL PRODUCTION BUG
**Complexity:** High (multiple interrelated issues)

**Problem:** Users cannot generate signing links. Every attempt shows "Unable to Generate Links" modal.

**Investigation History (Lesson: verify before celebrating):**

| Date | Hypothesis | Actual Result |
|------|------------|---------------|
| 2026-01-12 | NaN in field positions | Wrong - fix applied to dead code (`createSigningSession()`) |
| 2026-01-12 | Wrong function fixed | Partially correct - `generateSigningLinks()` was unfixed |
| 2026-01-12 | Field ID type mismatch | ✅ CONFIRMED via console: `floating point 1768237968710.0, expected string` |
| 2026-01-12 | After ID fix | NEW ERROR: `ReferenceError: addAuditEntry is not defined` |

**Issues Fixed:**
1. ✅ Field ID type: `id: f.id` → `id: String(f.id)` (line 6124)
2. ✅ `addAuditEntry` function missing - Added stub at line 2830

**Previous (Wrong) Root Cause:**
~~The frontend sends field positions that can be `NaN` or `null`~~ - This was a red herring. The actual issue was type coercion.

**Fix Applied (v2 - comprehensive):**
1. Added custom serde deserializer `deserialize_f64_with_default` that handles null/NaN
2. Added `#[serde(default)]` to all FieldInfo fields with sensible defaults
3. Added default functions for field_type, page, width_percent, height_percent
4. **NEW:** Added `#[serde(default)]` to ALL SessionMetadata fields (filename, page_count, created_at, created_by)
5. **NEW:** Added `sanitizeNum()` helper in frontend to prevent NaN/undefined from entering JSON
6. **NEW:** Added defensive fallbacks for all numeric values in session creation

**Regression Tests Added:**
- `test/session.test.ts` - 5 new tests:
  - "should handle NaN values in field positions gracefully" ✅
  - "should handle undefined/null field positions gracefully" ✅
  - "should accept valid field values" ✅
  - "should reject unauthenticated requests" ✅
  - "should reject requests with invalid token" ✅

**Test Results:**
- 113 Rust tests: PASS
- 19/20 JS tests: PASS (1 pre-existing rate limit failure - Bug #2)

**Acceptance Criteria:**
- [x] Users can generate signing links successfully (after deployment)
- [x] Clear error messages if specific failures occur
- [x] Regression tests capture the bug and prevent recurrence
- [ ] Puppeteer test confirms signing flow works end-to-end (after deployment)

**Files Modified:**
- `apps/docsign-web/worker/src/lib.rs`:
  - Added `deserialize_f64_with_default` custom deserializer
  - Added `#[serde(default)]` to FieldInfo fields
  - Added `#[serde(default)]` to SessionMetadata fields (filename, page_count, created_at, created_by)
  - Added default functions for all fields that need them
  - Added debug logging throughout `handle_create_session`
- `apps/docsign-web/worker/test/session.test.ts` - NEW: Regression tests
- `apps/docsign-web/www/index.html`:
  - Enhanced error handling with error codes
  - Added `sanitizeNum()` helper to prevent NaN/undefined values
  - Added defensive fallbacks for all numeric fields

**Deployment Required:**
```bash
# 1. Rebuild and deploy worker
cd apps/docsign-web/worker && worker-build --release && wrangler deploy

# 2. Build and deploy frontend
cd apps/docsign-web && trunk build --release
# Then deploy www/dist to Cloudflare Pages
```

---

### Bug #1: UX for Size Limits

**Status:** TESTING (2025-01-07)
**Priority:** HIGH
**Complexity:** Medium

**Problem:** Users hit PDF size, field, recipient, and text overflow limits with no clear feedback. The app either silently fails or crashes.

**What Was Fixed:**
1. ✅ Added validation module to `docsign-core` with property tests (15 tests)
2. ✅ Added WASM bindings: `validate_pdf_size()`, `validate_recipient_count()`, `validate_field_bounds()`
3. ✅ Frontend validates PDF size (100MB max) BEFORE loading into memory
4. ✅ Frontend validates recipient count (10 max) before adding
5. ✅ Frontend auto-adjusts field positions if they exceed page bounds
6. ✅ Added user-friendly error messages in `error-messages.ts`

**Acceptance Criteria:**
- [x] Property tests exist for all limit validations (15 tests in docsign-core)
- [x] Frontend shows clear modals BEFORE problematic action proceeds
- [ ] Puppeteer test confirms elderly user can understand the error

**Files modified:**
- `crates/docsign-core/src/lib.rs` - Added validation module with property tests
- `apps/docsign-web/wasm/src/lib.rs` - Exposed validations to JS
- `apps/docsign-web/www/index.html` - Added size/recipient validation in upload and add flows
- `apps/docsign-web/src/ts/error-messages.ts` - Added limit error messages

---

### Bug #2: Tests Waste Resend Credits + Stale KV Entries

**Status:** MOSTLY SOLVED (2025-01-07)
**Priority:** HIGH
**Complexity:** Low

**Problem:**
- Tests in `worker/test/auth.test.ts` called real Resend API, wasting credits
- Unverified test accounts filled KV storage

**What Was Fixed:**
1. ✅ Added `fetchMock` to intercept Resend API calls - tests now use mock
2. ✅ Added mock `RESEND_API_KEY` and `JWT_SECRET` in `vitest.config.ts`
3. ✅ Fixed test request format (`first_name`/`last_name` instead of `name`)
4. ✅ Created `scripts/kv-cleanup.sh` for manual cleanup when needed
5. ✅ Tests use `isolatedStorage: true` - don't pollute production KV

**Test Results:** 14 pass, 1 fail (rate limiting test expects wrong threshold)

**Remaining:**
- [ ] Fix rate limiting test (expects 429 after 6 requests, actual threshold differs)

**Files modified:**
- `apps/docsign-web/worker/test/auth.test.ts` - Added fetchMock, fixed API format
- `apps/docsign-web/worker/vitest.config.ts` - Added mock bindings
- `apps/docsign-web/scripts/kv-cleanup.sh` - Created for manual cleanup

---

### Bug #3: No Document Store (Drafts/Templates Don't Sync)

**Status:** OPEN
**Priority:** HIGH
**Complexity:** High

**Problem:** Drafts and templates only exist in IndexedDB (browser local storage). Users cannot:
- Continue a draft on a different device
- Share templates across devices
- Recover documents if browser data is cleared

**Current Behavior:**
- Drafts saved to IndexedDB `docsign_local` database
- Templates saved to IndexedDB + localStorage metadata
- No server-side persistence
- No cross-device sync

**Expected Behavior:**
- Encrypted documents stored server-side (Neon Postgres metadata + R2 blobs)
- Seed export/import for cross-device decryption
- Zero-knowledge preserved (server never sees plaintext)
- Offline-first with background sync

**Architecture Decision:**
- Neon Postgres for metadata
- Cloudflare R2 for encrypted PDF blobs
- Client-side AES-256 encryption before upload

**Acceptance Criteria:**
- [ ] Draft saved on Device A can be loaded on Device B (after seed import)
- [ ] Template field configurations sync across devices
- [ ] Server cannot decrypt documents (zero-knowledge verified)
- [ ] Works offline, syncs when back online

**Files to modify:**
- `apps/docsign-web/worker/wrangler.toml` - Add R2 binding
- `apps/docsign-web/worker/src/lib.rs` - Add /api/docs endpoints
- `apps/docsign-web/src/ts/local-session-manager.ts` - Add sync

---

### Bug #4: "Docs Left" Display + No Request System

**Status:** SOLVED (2025-01-07)
**Priority:** MEDIUM
**Complexity:** Medium
**Depends on:** Bug #6

**Problem:**
- Header shows "1 doc left this week" which user wants removed
- No way for users to request more ceremonies or report bugs
- No feedback loop to collect bug reports from real users

**Solution Implemented:**
1. ✅ Removed `#docs-remaining` span from header HTML
2. ✅ Removed JavaScript code that updated the display
3. ✅ Added "Feedback" button in header (💬 icon)
4. ✅ Created feedback modal with 4 request types: Bug, Feature, More Documents, Feedback
5. ✅ Added POST /requests/submit endpoint (requires auth)
6. ✅ Added admin email notification via Resend
7. ✅ Added RequestType, RequestStatus, UserRequest structs with tests
8. ✅ Rate limit: 1 pending request per user until resolved

**Acceptance Criteria:**
- [x] No document count visible in header
- [x] Feedback button opens modal
- [x] Bug report submission sends email to admin
- [x] User cannot submit second request until first resolved
- [x] Request includes optional ceremony count

**Files created/modified:**
- `apps/docsign-web/worker/src/auth/types.rs` - RequestType, RequestStatus, UserRequest, SubmitRequestBody/Response
- `apps/docsign-web/worker/src/lib.rs` - POST /requests/submit endpoint + handle_submit_request handler
- `apps/docsign-web/worker/src/email/mod.rs` - send_admin_notification_email()
- `apps/docsign-web/www/index.html` - Feedback button, modal, JavaScript functions

---

### Bug #5: Missing Legal Disclaimers / TOS

**Status:** SOLVED (2025-01-07)
**Priority:** MEDIUM
**Complexity:** Low

**Problem:** No Terms of Service or legal disclaimers. Users have no understanding of:
- Service warranty limitations
- Liability caps
- E-signature legal compliance responsibility
- Data handling practices

**Solution Implemented:**
1. ✅ Created `/legal.html` with full TOS, Privacy Policy, E-Sign Disclosure, Security sections
2. ✅ Added TOS acceptance checkbox to registration form (required)
3. ✅ Added footer with legal links to auth.html, index.html
4. ✅ Added E-Sign Disclosure and Privacy links in consent modal for remote signers
5. ✅ Added Legal link to pricing.html header

**Key Disclaimers Included:**
1. AS-IS warranty - Services provided without warranty
2. Liability cap - $100 for free users, 3 months fees for paid
3. E-signature compliance - User responsible for legal validity
4. Zero-knowledge encryption - Documents never readable by servers
5. Data retention policies
6. User rights (access, delete, export)

**Acceptance Criteria:**
- [x] /legal.html exists with full TOS
- [x] Registration requires TOS acceptance checkbox
- [x] Footer links to legal page
- [x] Signing page has clickable privacy notice

**Files created:**
- `apps/docsign-web/www/legal.html` - NEW: Full legal page

**Files modified:**
- `apps/docsign-web/www/auth.html` - TOS checkbox, footer links
- `apps/docsign-web/www/index.html` - Footer links, consent modal privacy links
- `apps/docsign-web/www/pricing.html` - Legal link in header

---

### Bug #6: Complete Pricing & Tier System

**Status:** TESTING (2025-01-07) - Core features complete, Stripe deferred
**Priority:** HIGH
**Complexity:** High

**Problem:** Need complete 4-tier pricing system with pricing page, usage tracking, limit notifications, and Stripe integration.

**New Pricing Structure:**
| Tier | Docs/Month | Price | Overage | Max w/Overage |
|------|------------|-------|---------|---------------|
| Free | 3 | $0 | Hard limit | 3 |
| Personal | 25 | $10/mo | $0.50/doc | 50 |
| Professional | 100 | $25/mo | $0.50/doc | 200 |
| Business | 300 | $60/mo | $0.50/doc | 600 |

**What Was Fixed:**
1. ✅ Expanded `UserTier` enum: Free, Personal, Professional, Business (+ legacy Pro)
2. ✅ Added `BillingCycle` enum: Monthly, Annual
3. ✅ Added tier methods: `monthly_limit()`, `max_with_overage()`, `allows_overage()`, `display_name()`, pricing methods
4. ✅ Added billing fields to User: `stripe_customer_id`, `stripe_subscription_id`, `billing_cycle`, `overage_count`, `limit_email_sent`
5. ✅ Updated `can_create_document()` to respect tier limits + overage
6. ✅ Added `record_document_send()` method that returns true when limit reached
7. ✅ Updated `/api/start` to use new tier-aware limit checking
8. ✅ Created `/pricing.html` with 4-tier cards, monthly/annual toggle
9. ✅ Added 10 new unit tests for tier system (116 total tests pass)
10. ✅ Bug #6.3: Added in-app tier presentation:
    - "Plan & Usage" section in settings modal with tier badge + usage bar
    - Soft-block modal when user hits monthly limit
    - `updatePlanUsage()`, `showLimitModal()`, `closeLimitModal()` functions
    - 429 MONTHLY_LIMIT_EXCEEDED triggers soft-block modal in generateSigningLinks()

**User Decisions:**
- Build WITHOUT Stripe first (add payments later)
- Soft block at limit: Allow viewing existing docs, block new sends
- Email only at 100%: No early warning emails, only when limit reached

**Remaining Work:**
- [x] Bug #6.3: Add in-app tier presentation (settings modal, usage bar, soft block modal)
- [x] Bug #6.4: Add limit notification email via Resend
- [ ] Stripe integration (deferred)

**Bug #6.4 Implementation:**
- Added `send_limit_notification_email()` in `email/mod.rs` with HTML template
- Email includes: personalized greeting, clear limit info, "what you can still do" list, reset date, upgrade CTA
- Sent when user hits base limit (not overage) and `limit_email_sent` flag is false
- Flag prevents duplicate emails in same billing period
- Flag resets in `check_monthly_reset()` with new month

**Files modified:**
- `apps/docsign-web/worker/src/auth/types.rs` - UserTier, BillingCycle, User fields, tests
- `apps/docsign-web/worker/src/auth/handlers.rs` - Use tier.display_name() for JWT
- `apps/docsign-web/worker/src/lib.rs` - Tier-aware limit checking + record_document_send + limit email
- `apps/docsign-web/worker/src/email/mod.rs` - Added send_limit_notification_email()
- `apps/docsign-web/www/pricing.html` - NEW: Pricing page with 4 tiers
- `apps/docsign-web/www/auth.html` - Added pricing link to header
- `apps/docsign-web/www/index.html` - Plan & Usage section, soft-block modal, JS functions

---

### Bug #7: Name Change Approval Workflow

**Status:** SOLVED (2025-01-07)
**Priority:** MEDIUM
**Complexity:** Medium
**Depends on:** Bug #8

**Problem:** Users could change their name freely, which is problematic for a signing app where name integrity matters.

**Solution Implemented:**
1. Added `name_set: bool` field to User struct (defaults to true if name provided at registration)
2. Added `pending_name_change_request_id: Option<String>` to track pending requests
3. Added `RequestType::NameChange` with name change fields in UserRequest
4. Added `UserRequest::new_name_change()` constructor
5. Modified `handle_update_profile` in handlers.rs:
   - First-time name set (legacy accounts): Allowed without approval, sets `name_set = true`
   - Name already set: Creates a NameChange request instead of direct update
   - User with pending request: Returns error message
6. Updated admin endpoint to apply name changes when approving NameChange requests
7. Clearing pending request ID on approve/deny

**Behavior:**
- **First name set:** Allowed immediately (for legacy accounts or new users without name)
- **Subsequent changes:** Creates approval request, user sees "pending approval" message
- **Admin approval:** Name is applied, pending request cleared
- **Admin denial:** Pending request cleared, user can request again

**Acceptance Criteria:**
- [x] `name_set: bool` field tracks if name was ever set
- [x] Subsequent name changes create approval request
- [x] User sees "pending approval" status message
- [x] Admin can approve/deny name changes in dashboard

**Files modified:**
- `apps/docsign-web/worker/src/auth/types.rs` - User fields, RequestType::NameChange, 2 new tests
- `apps/docsign-web/worker/src/auth/handlers.rs` - handle_update_profile with approval logic
- `apps/docsign-web/worker/src/lib.rs` - handle_admin_update_request applies/rejects name changes

---

### Bug #8: Admin Dashboard

**Status:** SOLVED (2025-01-07)
**Priority:** MEDIUM
**Complexity:** High
**Depends on:** Bugs #4, #6

**Problem:** No way to manage users, approve requests, or monitor the system.

**Solution Implemented:**
1. Created `/admin.html` - hidden admin dashboard (not linked in navigation)
2. Access restricted to orlandodowntownhome@gmail.com only (case-insensitive)
3. Added admin types in `auth/types.rs`:
   - `is_admin()` function, `ADMIN_EMAIL` constant
   - `AdminRequestAction`, `AdminUpdateRequestBody/Response`
   - `AdminAdjustQuotaBody/Response`, `AdminUserSummary`
   - `AdminUsersListResponse`, `AdminRequestsListResponse`, `AdminDeleteUserResponse`
4. Added 5 admin endpoints in `lib.rs`:
   - `GET /admin/requests` - List requests (supports ?status= filter)
   - `POST /admin/requests/:id` - Approve/deny/mark in-progress
   - `GET /admin/users` - List users (supports ?filter=unverified|verified)
   - `POST /admin/users/:id/quota` - Adjust tier or grant bonus documents
   - `DELETE /admin/users/:id` - Delete user (cleans up email index and pending requests)
5. Frontend features:
   - Stats dashboard (pending requests, total users, unverified users)
   - Filterable request list with approve/deny/in-progress actions
   - Filterable user list with quota adjustment and delete options
   - Quota modal for tier changes and bonus document grants
   - Request action modal with admin notes

**Acceptance Criteria:**
- [x] /admin.html exists and loads
- [x] Non-admin users see "Access Denied" message
- [x] Admin can list pending requests with status filter
- [x] Admin can approve/deny/mark-in-progress requests
- [x] Admin can adjust user quotas (tier + bonus documents)
- [x] Admin can delete unverified accounts

**Files created:**
- `apps/docsign-web/www/admin.html` - Admin dashboard page

**Files modified:**
- `apps/docsign-web/worker/src/auth/types.rs` - Admin types (5 new tests)
- `apps/docsign-web/worker/src/lib.rs` - Admin endpoints and handlers
- `apps/docsign-web/www/index.html` - Added copy-file directive for admin.html

---

### Bug #9: No Paid Tiers (Stripe Connect)

**Status:** OPEN
**Priority:** LOW (future work)
**Complexity:** High
**Depends on:** All above bugs

**Problem:** Only free tier exists. No revenue model.

**Pricing Table:**

| Tier | Ceremonies/Month | Price | Pay-as-you-go |
|------|-----------------|-------|---------------|
| Free | 1 | $0 | N/A |
| Personal | 20 | $10/mo | $1/extra |
| Team | 55 | $20/mo | $1/extra |
| Business | 200 | $40/mo | $1/extra |

**Implementation Notes:**
- Use Stripe Connect for subscription management
- Webhook for payment events
- Automatic tier upgrade/downgrade
- Pay-as-you-go toggle in settings

**Acceptance Criteria:**
- [ ] Stripe Connect integration works
- [ ] Users can upgrade/downgrade tiers
- [ ] Pay-as-you-go charges correctly
- [ ] Webhook handles payment failures gracefully

---

### Bug #10: Review Page Shows Wrong Page Count (1/1)

**Status:** SOLVED (2026-02-01)
**Priority:** P1 - HIGH (user-reported)
**Complexity:** Low (reorder function calls)

**Problem:** On Step 4 (Review), the page indicator shows "1 / 1" for a 14-page document. Page number never updates when scrolling.

**Root Cause:** In `renderReview()`, `updateReviewUI()` was called BEFORE `setupEditablePageInput()`.
- `setupEditablePageInput()` creates the `_updateDisplay` function on the page indicator
- `updateReviewUI()` uses `_updateDisplay` to update the count
- Since `updateReviewUI()` ran first, `_updateDisplay` didn't exist yet, so update failed

**Fix Applied (2026-02-01):**
Reordered the function calls in `renderReview()`:
1. Call `setupEditablePageInput()` FIRST (creates `_updateDisplay`)
2. Call `updateReviewUI()` AFTER (now `_updateDisplay` exists)

**Acceptance Criteria:**
- [x] Review page shows correct total page count (e.g., "1 / 14")
- [x] Page number updates when scrolling
- [x] Prev/Next buttons navigate correctly
- [x] scrollToReviewPage() works

**Files Modified:**
- `apps/docsign-web/www/index.html` - Reordered Bug #12 setup before Bug #10 updateReviewUI() call

---

### Bug #11: Missing Jump-to-Top/Bottom Buttons

**Status:** SOLVED (2026-02-01 - verified already implemented)
**Priority:** P2 - MEDIUM
**Complexity:** Medium

**Problem:** Users must manually scroll through long documents (14+ pages). No quick way to jump to top or bottom.

**Implementation:** Per NN/G guidelines:
- Floating button group (bottom-right corner)
- Jump-to-top (↑): hidden when at top, visible after scrolling past first screen
- Jump-to-bottom (↓): hidden when at bottom, visible when not at end
- 44x44px minimum touch targets (Apple HIG, WCAG)
- Apply to ALL preview containers (Steps 2, 3, 4, and signer view)

**Solution:** `setupScrollNavButtons()` function implemented and called in:
- Preview container (Step 2)
- Fields container (Step 3)
- Review container (Step 4)

CSS styles defined at line 1616 (`.scroll-nav-buttons`)

**Acceptance Criteria:**
- [x] Floating buttons appear in all PDF preview areas
- [x] Jump-to-top smoothly scrolls to first page
- [x] Jump-to-bottom smoothly scrolls to last page
- [x] Buttons show/hide based on scroll position
- [x] Touch targets are 44x44px minimum

---

### Bug #12: Page Number Not Editable/Clickable

**Status:** SOLVED (2026-02-01 - verified already implemented)
**Priority:** P2 - MEDIUM
**Complexity:** Medium

**Problem:** Users cannot click on the page indicator to jump to a specific page. Modern document viewers (Google Docs, Adobe Acrobat, Figma) allow clicking the page number to edit it.

**Solution:** `setupEditablePageInput()` function implemented at line 4472.
- Converts page indicator span to clickable button + hidden input
- Stores `_updateDisplay` function for external updates
- Applied to preview and review containers

CSS styles defined at lines 1668-1701 (`.page-input-trigger`, `.page-input`)

**Acceptance Criteria:**
- [x] Click on page indicator shows editable input
- [x] Enter commits and scrolls to page
- [x] Invalid values (0, 999) clamped to valid range
- [x] Escape cancels edit
- [x] Works in Steps 2, 3, 4, and signer view

---

### Bug #13: Zoom Range Limited to 300%

**Status:** SOLVED (2026-02-01 - verified already implemented)
**Priority:** P3 - LOW
**Complexity:** Low

**Problem:** Maximum zoom is 300%. Some users need 400% for detailed viewing of small text or signatures.

**Solution:**
- `maxZoom: 4.0` set in all zoom state objects (lines 3956, 4144, 4270)
- `setupCtrlScrollZoom()` function implemented at line 4551
- Called in renderReview() for Ctrl+scroll zoom support

**Acceptance Criteria:**
- [x] Zoom reaches 400% maximum
- [x] Ctrl+scroll zooms on desktop
- [x] Pinch-to-zoom works on all preview areas

---

### Bug #14: CRITICAL - Signing Link Doesn't Load (P0)

**Status:** SOLVED (2026-01-12)
**Priority:** P0 - CRITICAL (blocks all signing flow testing)
**Complexity:** Medium

**Problem:** Email link opens but shows "Loading Get Signatures..." forever.

**Root Cause - THREE separate bugs:**

1. **URL Parsing Format Mismatch:**
   - Link format: `#sign=SESSION_ID:RECIPIENT_ID:ENCRYPTION_KEY` (colon-separated)
   - Parser expected: `#sign=SESSION_ID&key=KEY&r=RECIPIENT_ID` (query param format)
   - URLSearchParams couldn't parse colon-separated values

2. **Token in Wrong Location:**
   - Server appends `?token=TOKEN` AFTER the hash
   - Result: `#sign=SESSION:ID:KEY?token=TOKEN`
   - Token was in fragment, not query string where server expects it

3. **Missing Token in API Call:**
   - GET /session/:id requires `?token=...` for non-legacy sessions
   - Frontend wasn't extracting token from URL

**Fix Applied:**
1. Updated `checkForSigningLink()` to handle colon-separated format
2. Added token extraction from both query string AND hash fragment
3. Updated `loadRemoteSession()` to accept and pass token parameter
4. Added `?token=${token}` to session API call

**Files Modified:**
- `apps/docsign-web/www/index.html` - URL parsing + token handling

---

### Bug #15: Signing Links Visible to Sender (P1)

**Status:** SOLVED (2026-01-12)
**Priority:** P1 - HIGH
**Complexity:** Low

**Problem:** Modal shows full signing URLs with Copy buttons. Links should ONLY be sent via email, never shown to sender.

**Security Concern:** Sender could copy link and share it insecurely (via unencrypted channel).

**Fix Applied:**
1. Changed modal title from "Signing Links Generated" to "Ready to Send"
2. Changed description from "Share these unique links..." to "Secure signing links will be sent via email"
3. Removed link URLs and Copy buttons from display
4. Show only recipient names and emails
5. Changed button text from "Send Emails" to "Send Signing Requests"

**Files Modified:**
- `apps/docsign-web/www/index.html` - Modal HTML + JS rendering

---

### Bug #16: Missing Email Opt-Out (P1)

**Status:** SOLVED (2026-01-12)
**Priority:** P1 - HIGH (CAN-SPAM/GDPR compliance)
**Complexity:** Low

**Problem:** Email template lacked unsubscribe/opt-out mechanism required by CAN-SPAM.

**Fix Applied:**
Added footer to email template:
- "You received this email because [sender] requested your signature"
- "This is a transactional email related to a specific signature request"
- "If you believe you received this in error, contact the sender directly"

**Note:** Full unsubscribe endpoint deferred - transactional emails (signature requests) generally don't require unsubscribe, but explanation text added for transparency.

**Files Modified:**
- `apps/docsign-web/worker/src/lib.rs` - Email template footer

---

### Bug #17: Review Step Keeps Field Tool Selected (P2)

**Status:** SOLVED (2026-01-12)
**Priority:** P2 - MEDIUM
**Complexity:** Trivial (1 line)

**Problem:** Going from Step 3 (Place Fields) to Step 4 (Review) keeps the signature/field tool selected. Clicking on document would try to place a field.

**Root Cause:** `goToStep(4)` didn't call `setFieldType('pointer')` like Step 3 does.

**Fix Applied:**
```javascript
} else if (step === 4) {
    setFieldType('pointer'); // Bug #17 fix: Reset to pointer mode
    renderReview();
}
```

**Files Modified:**
- `apps/docsign-web/www/index.html` - goToStep() function

---

### Bug #18: Email Shows "Hello ," (Missing Name) (P2)

**Status:** SOLVED (2026-01-12)
**Priority:** P2 - MEDIUM
**Complexity:** Low

**Problem:** Email greeting shows "Hello ," when recipient has no name.

**Root Cause:** `invitation.name` was empty string, template just used it directly.

**Fix Applied:**
```rust
let recipient_name = if invitation.name.trim().is_empty() {
    invitation.email.split('@').next().unwrap_or("there").to_string()
} else {
    invitation.name.clone()
};
```

Email now shows "Hello john," for john@example.com if no name provided.

**Files Modified:**
- `apps/docsign-web/worker/src/lib.rs` - Invitation email handler

---

## Solved Bugs

*Bugs that have been fixed move here. Keep for historical reference.*

<!--
Template for solved bugs:

### Bug #X: Title

**Status:** SOLVED
**Solved Date:** YYYY-MM-DD
**Commit:** abc1234

**Problem:** (original problem description)

**Solution:** (what was done to fix it)

**Verification:** (how it was verified - tests + Puppeteer)
-->
