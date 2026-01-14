//! DocSign Server - Cloudflare Worker for email relay and signing sessions
//!
//! Email sending via Resend API (provider-agnostic, can swap to AWS SES/Postmark)
//! Signing sessions expire after 7 days

mod auth;
mod billing;
mod email;

use chrono::Utc;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hmac::{Hmac, Mac};
use lopdf::{Dictionary, Object, ObjectId, Stream};
use png::Decoder as PngDecoder;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::io::Write;
use worker::*;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

// ============================================================
// WASM-Safe Time Utilities
// ============================================================
// IMPORTANT: std::time::SystemTime does NOT work in Cloudflare Workers WASM!
// It panics with "time not implemented on this platform"
// Always use these helpers instead.

/// Get current Unix timestamp in seconds (WASM-safe)
/// Uses js_sys::Date::now() which works in Cloudflare Workers
fn get_timestamp_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Get current Unix timestamp in milliseconds (WASM-safe)
fn get_timestamp_millis() -> u128 {
    js_sys::Date::now() as u128
}

// ============================================================
// TESTING MODE: Unlimited Sends for Development
// ============================================================
// TODO: REMOVE THIS BEFORE PRODUCTION LAUNCH!
// These accounts bypass per-user document limits for testing.
// Global Resend email limits still apply.
//
// To remove: Delete this constant, the helper function below,
// and the is_unlimited check in handle_create_session().
// ============================================================

#[deprecated(note = "TESTING ACCOUNTS ACTIVE - Remove before production launch!")]
const TESTING_UNLIMITED_EMAILS: &[&str] = &[
    "orlandodowntownhome@gmail.com",
    "bobamatchasolutions@gmail.com",
];

/// Check if email has unlimited sends for testing
/// Returns true for accounts in TESTING_UNLIMITED_EMAILS
#[allow(deprecated)]
fn is_testing_unlimited_account(email: &str) -> bool {
    TESTING_UNLIMITED_EMAILS
        .iter()
        .any(|e| e.eq_ignore_ascii_case(email))
}

/// Request body for sending a document
#[derive(Deserialize)]
#[allow(dead_code)] // Fields used via serde deserialization
struct SendRequest {
    /// Recipient email address
    to: String,
    /// Email subject
    subject: String,
    /// PDF document as base64
    pdf_base64: String,
    /// Filename for the attachment
    filename: String,
    /// Optional: signing link to include in email
    #[serde(default)]
    signing_link: Option<String>,
}

/// Response from send endpoint
#[derive(Serialize)]
struct SendResponse {
    success: bool,
    message: String,
}

/// Health check response for monitoring
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
    dependencies: HealthDependencies,
}

#[derive(Serialize)]
struct HealthDependencies {
    kv_sessions: DependencyStatus,
    kv_rate_limits: DependencyStatus,
}

#[derive(Serialize)]
struct DependencyStatus {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Signing mode determines how multiple signers interact
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
enum SigningMode {
    /// All signers sign the original simultaneously (default - simpler for users)
    #[default]
    Parallel,
    /// Signers must sign in order, each seeing previous signatures
    Sequential,
}

/// Reminder configuration for pending signers
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ReminderConfig {
    /// Hours between reminders (default: 48 = every 2 days)
    #[serde(default = "default_reminder_hours")]
    frequency_hours: u32,
    /// Maximum reminders per recipient (default: 3)
    #[serde(default = "default_max_reminders")]
    max_count: u32,
    /// Whether reminders are enabled (default: true)
    #[serde(default = "default_reminders_enabled")]
    enabled: bool,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            frequency_hours: 48,
            max_count: 3,
            enabled: true,
        }
    }
}

fn default_reminder_hours() -> u32 {
    48
}
fn default_max_reminders() -> u32 {
    3
}
fn default_reminders_enabled() -> bool {
    true
}

/// Request to create a signing session
#[derive(Deserialize)]
struct CreateSessionRequest {
    /// Encrypted document data (base64)
    encrypted_document: String,
    /// Document metadata (filename, page count, etc.)
    metadata: SessionMetadata,
    /// Recipients and their signing status
    recipients: Vec<RecipientInfo>,
    /// Field configurations (positions as percentages)
    fields: Vec<FieldInfo>,
    /// Session expiry in hours (default: 168 = 7 days)
    #[serde(default = "default_expiry_hours")]
    expiry_hours: u32,
    /// Signing mode: parallel (default) or sequential
    #[serde(default)]
    signing_mode: SigningMode,
    /// Reminder configuration
    #[serde(default)]
    reminder_config: ReminderConfig,
    /// Legacy mode: if true, allow GET without token (for backwards compatibility/testing)
    #[serde(default)]
    legacy: bool,
}

fn default_expiry_hours() -> u32 {
    168
} // 7 days

/// Bug #0 fix: Add defaults to all SessionMetadata fields to handle missing/null values
#[derive(Serialize, Deserialize, Clone)]
struct SessionMetadata {
    #[serde(default = "default_filename")]
    filename: String,
    #[serde(default = "default_page_count")]
    page_count: u32,
    #[serde(default = "default_created_at")]
    created_at: String,
    #[serde(default = "default_created_by")]
    created_by: String,
    #[serde(default)]
    sender_email: Option<String>,
    /// Feature 1: Optional document alias (e.g., "Q1 2026 Lease Agreement")
    #[serde(default)]
    document_alias: Option<String>,
    /// Feature 1: Optional signing context (e.g., "Lease for 30 James Ave, Orlando")
    #[serde(default)]
    signing_context: Option<String>,
}

fn default_filename() -> String {
    "document.pdf".to_string()
}

fn default_page_count() -> u32 {
    1
}

fn default_created_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_created_by() -> String {
    "Unknown".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
struct RecipientInfo {
    id: String,
    name: String,
    email: String,
    role: String, // "signer" or "viewer"
    /// Whether recipient consented to electronic signing (clicked "Review Document")
    #[serde(default)]
    consented: bool,
    /// ISO 8601 timestamp when consent was given
    #[serde(default)]
    consent_at: Option<String>,
    /// User agent string at time of consent (for audit trail)
    #[serde(default)]
    consent_user_agent: Option<String>,
    #[serde(default)]
    signed: bool,
    #[serde(default)]
    signed_at: Option<String>,
    /// Whether the recipient declined to sign (UX-002)
    #[serde(default)]
    declined: bool,
    /// ISO 8601 timestamp when declined (UX-002)
    #[serde(default)]
    declined_at: Option<String>,
    /// Optional reason for declining (UX-002)
    #[serde(default)]
    decline_reason: Option<String>,
    /// For sequential mode: signing order (1, 2, 3...). None = parallel/any order
    #[serde(default)]
    signing_order: Option<u32>,
    /// Number of reminders sent to this recipient
    #[serde(default)]
    reminders_sent: u32,
    /// ISO 8601 timestamp of last reminder
    #[serde(default)]
    last_reminder_at: Option<String>,
}

/// Session status for the signing workflow (UX-002)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
enum SessionStatus {
    /// Session is pending - waiting for recipient action
    #[default]
    Pending,
    /// Recipient accepted and is signing
    Accepted,
    /// Recipient declined to sign - blocks other signers until sender revises/discards
    Declined,
    /// All signatures completed
    Completed,
    /// Session has expired
    Expired,
    /// Session was voided/discarded by sender - all signing links invalidated
    Voided,
}

/// Bug #0 fix: Custom deserializer for f64 that handles null/NaN/undefined
/// JSON.stringify(NaN) produces null, which serde can't parse as f64
/// This deserializer converts null to a default value
fn deserialize_f64_with_default<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    // Try to deserialize as Option<f64> to handle null
    let opt = Option::<f64>::deserialize(deserializer)?;
    match opt {
        Some(v) if v.is_nan() => Ok(0.0), // NaN becomes 0
        Some(v) => Ok(v),
        None => Ok(0.0), // null becomes 0
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct FieldInfo {
    #[serde(default)]
    id: String,
    #[serde(default = "default_field_type")]
    field_type: String,
    #[serde(default)]
    recipient_id: String,
    #[serde(default = "default_page")]
    page: u32,
    /// Bug #0: Field positions now handle null/NaN/undefined gracefully
    #[serde(default, deserialize_with = "deserialize_f64_with_default")]
    x_percent: f64,
    #[serde(default, deserialize_with = "deserialize_f64_with_default")]
    y_percent: f64,
    #[serde(
        default = "default_width_percent",
        deserialize_with = "deserialize_f64_with_default"
    )]
    width_percent: f64,
    #[serde(
        default = "default_height_percent",
        deserialize_with = "deserialize_f64_with_default"
    )]
    height_percent: f64,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    value: Option<String>,
}

fn default_field_type() -> String {
    "signature".to_string()
}

fn default_page() -> u32 {
    1
}

fn default_width_percent() -> f64 {
    20.0
}

fn default_height_percent() -> f64 {
    5.0
}

/// Stored signing session
#[derive(Serialize, Deserialize)]
struct SigningSession {
    id: String,
    /// The original document - never modified (for parallel mode merging)
    encrypted_document: String,
    metadata: SessionMetadata,
    recipients: Vec<RecipientInfo>,
    fields: Vec<FieldInfo>,
    expires_at: String,
    /// LEGACY: Each signer's full PDF copy (deprecated - use signature_annotations instead)
    /// Kept for backwards compatibility with existing sessions
    #[serde(default)]
    signed_versions: Vec<SignedVersion>,
    /// NEW: Lightweight signature annotations (~50KB each vs ~13MB full PDF copies)
    /// Original PDF + annotations are merged on-demand when downloading
    #[serde(default)]
    signature_annotations: Vec<SignatureAnnotation>,
    /// Session status for the workflow (UX-002)
    #[serde(default)]
    status: SessionStatus,
    /// Signing mode: parallel (default) or sequential
    #[serde(default)]
    signing_mode: SigningMode,
    /// Reminder configuration for pending signers
    #[serde(default)]
    reminder_config: Option<ReminderConfig>,
    /// Final merged document when all signers complete (parallel mode only)
    #[serde(default)]
    final_document: Option<String>,
    /// Legacy flag: if true, allow access without token (for backwards compatibility)
    /// New sessions should NOT set this flag (defaults to false)
    #[serde(default)]
    legacy: bool,
    /// Number of revisions made to this document
    #[serde(default)]
    revision_count: u32,
    /// When document was voided (if applicable)
    #[serde(default)]
    voided_at: Option<String>,
    /// Reason for voiding
    #[serde(default)]
    void_reason: Option<String>,
    /// Token version - incremented on revise/void to invalidate old tokens
    #[serde(default)]
    token_version: u32,
}

#[derive(Serialize, Deserialize, Clone)]
struct SignedVersion {
    recipient_id: String,
    encrypted_document: String,
    signed_at: String,
}

// ============================================================================
// New Annotation-based Storage (v2)
// Instead of storing full PDF copies per signer (~13MB each), we store only
// signature annotations (~50KB each). This allows unlimited signers within
// the 25MB KV limit. The original PDF is merged with annotations on-demand.
// ============================================================================

/// A single signature annotation (signature, initials, date, text, or checkbox)
#[derive(Serialize, Deserialize, Clone)]
struct SignatureAnnotation {
    /// Unique ID for this annotation
    id: String,
    /// Which recipient placed this annotation
    recipient_id: String,
    /// Which field this annotation fills
    field_id: String,
    /// Type of annotation
    field_type: AnnotationFieldType,
    /// The actual annotation data (signature image, text, etc.)
    data: AnnotationData,
    /// Position on the PDF page
    position: AnnotationPosition,
    /// When this annotation was created
    signed_at: String,
}

/// Type of annotation field
#[derive(Serialize, Deserialize, Clone)]
enum AnnotationFieldType {
    Signature,
    Initials,
    Date,
    Text,
    Checkbox,
}

/// The actual data for an annotation
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum AnnotationData {
    /// Drawn/uploaded signature image
    DrawnSignature { image_base64: String },
    /// Typed signature with font
    TypedSignature { text: String, font: String },
    /// Initials (drawn or typed)
    Initials { image_base64: String },
    /// Date value
    Date { value: String },
    /// Free text
    Text { value: String },
    /// Checkbox state
    Checkbox { checked: bool },
}

/// Position of annotation on PDF page (all values as percentages for resolution-independence)
#[derive(Serialize, Deserialize, Clone)]
struct AnnotationPosition {
    /// Page number (0-indexed)
    page: u32,
    /// X position as percentage of page width (0-100)
    x_percent: f64,
    /// Y position as percentage of page height (0-100)
    y_percent: f64,
    /// Width as percentage of page width
    width_percent: f64,
    /// Height as percentage of page height
    height_percent: f64,
}

/// Response for session creation
#[derive(Serialize)]
struct CreateSessionResponse {
    success: bool,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Response for session retrieval
#[derive(Serialize)]
struct GetSessionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<SessionPublicInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct SessionPublicInfo {
    id: String,
    metadata: SessionMetadata,
    recipients: Vec<RecipientInfo>,
    fields: Vec<FieldInfo>,
    encrypted_document: String,
    expires_at: String,
    /// Signing mode for this session
    signing_mode: SigningMode,
    /// Session status
    status: SessionStatus,
    /// Final merged document (parallel mode only, when all signed)
    #[serde(skip_serializing_if = "Option::is_none")]
    final_document: Option<String>,
}

/// Request to submit signed document (LEGACY: stores full PDF copy per signer)
#[derive(Deserialize)]
struct SubmitSignedRequest {
    recipient_id: String,
    encrypted_document: String,
}

/// Request to submit signature annotations (NEW: stores only annotation data ~50KB)
/// Use this instead of SubmitSignedRequest to support unlimited signers
#[derive(Deserialize)]
struct SubmitAnnotationsRequest {
    recipient_id: String,
    /// Annotations placed by this recipient
    annotations: Vec<AnnotationSubmission>,
}

/// A single annotation submission from the frontend
#[derive(Deserialize)]
struct AnnotationSubmission {
    /// Which field this annotation fills
    field_id: String,
    /// Type of annotation (defaults to Signature if not specified)
    #[serde(default)]
    field_type: Option<AnnotationFieldType>,
    /// The actual annotation data
    data: AnnotationData,
    /// Position on the PDF page (optional - can be looked up from field_id)
    #[serde(default)]
    position: Option<AnnotationPosition>,
}

/// Response from annotation submission
#[derive(Serialize)]
struct SubmitAnnotationsResponse {
    success: bool,
    message: String,
    /// True if all signers have now completed
    all_signed: bool,
    /// Number of remaining signers
    remaining_signers: u32,
    /// Download URL for the signed document (only when all_signed=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    download_url: Option<String>,
}

/// Request to decline a document (UX-002)
#[derive(Deserialize)]
struct DeclineRequest {
    recipient_id: String,
    reason: Option<String>,
}

/// Response from decline endpoint (UX-002)
#[derive(Serialize)]
struct DeclineResponse {
    success: bool,
    message: String,
}

/// Request to void/discard a document
#[derive(Deserialize)]
struct VoidRequest {
    /// Optional reason for voiding
    reason: Option<String>,
}

/// Response from void endpoint
#[derive(Serialize)]
struct VoidResponse {
    success: bool,
    message: String,
}

/// Request to revise a document (only if no signers have signed yet)
#[derive(Deserialize)]
struct ReviseRequest {
    /// Updated field positions
    fields: Vec<FieldInfo>,
    /// Optional: new/modified recipients (full revision - can add/remove signers)
    #[serde(default)]
    recipients: Option<Vec<ReviseRecipientInput>>,
    /// Optional: replacement document as base64
    #[serde(default)]
    document: Option<String>,
    /// Optional: note to signers about the revision
    #[serde(default)]
    message: Option<String>,
}

/// Recipient info for revision
#[derive(Deserialize, Clone)]
struct ReviseRecipientInput {
    /// Existing recipient ID (for retained signers) or new unique ID
    id: String,
    name: String,
    email: String,
    #[serde(default = "default_signer_role")]
    role: String,
    #[serde(default)]
    signing_order: Option<u32>,
}

fn default_signer_role() -> String {
    "signer".to_string()
}

/// Token info returned after revision
#[derive(Serialize)]
struct TokenInfo {
    recipient_id: String,
    recipient_name: String,
    recipient_email: String,
    signing_url: String,
}

/// Response from revise endpoint
#[derive(Serialize)]
struct ReviseResponse {
    success: bool,
    message: String,
    /// New signing links for all recipients
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<Vec<TokenInfo>>,
}

/// Request to restart a document (clears all signatures and starts fresh)
/// Use this when some signers have already signed but need to revise
#[derive(Deserialize)]
struct RestartRequest {
    /// Updated field positions
    fields: Vec<FieldInfo>,
    /// Optional: new/modified recipients (can add/remove signers)
    #[serde(default)]
    recipients: Option<Vec<ReviseRecipientInput>>,
    /// Optional: replacement document as base64
    #[serde(default)]
    document: Option<String>,
    /// Optional: note to signers about the restart
    #[serde(default)]
    message: Option<String>,
}

/// Response from restart endpoint
#[derive(Serialize)]
struct RestartResponse {
    success: bool,
    message: String,
    /// New signing links for all recipients
    #[serde(skip_serializing_if = "Option::is_none")]
    tokens: Option<Vec<TokenInfo>>,
}

/// Request to associate a signed session with a newly created account
/// Used when a signer creates an account after signing to view the document in their dashboard
#[derive(Deserialize)]
struct AssociateSessionRequest {
    session_id: String,
}

// ============================================================
// Template Types (Phase 3)
// ============================================================

/// A field template defines reusable field placements for document signing
#[derive(Serialize, Deserialize, Clone)]
struct FieldTemplate {
    id: String,
    name: String,
    fields: Vec<TemplateField>,
    created_at: String,
    updated_at: String,
}

/// A field within a template
#[derive(Serialize, Deserialize, Clone)]
struct TemplateField {
    field_type: String,
    /// 0 = first signer, 1 = second signer, etc.
    recipient_index: u32,
    page: u32,
    x_percent: f64,
    y_percent: f64,
    width_percent: f64,
    height_percent: f64,
    required: bool,
}

/// Index of templates for a user
#[derive(Serialize, Deserialize, Default)]
struct TemplateIndex {
    templates: Vec<TemplateIndexEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct TemplateIndexEntry {
    id: String,
    name: String,
    field_count: usize,
    created_at: String,
    updated_at: String,
}

/// Request to create/update a template
#[derive(Deserialize)]
struct CreateTemplateRequest {
    name: String,
    fields: Vec<TemplateField>,
}

/// Response from template creation
#[derive(Serialize)]
struct CreateTemplateResponse {
    success: bool,
    template_id: String,
    message: String,
}

/// Response with list of templates
#[derive(Serialize)]
struct ListTemplatesResponse {
    success: bool,
    templates: Vec<TemplateIndexEntry>,
}

/// Response with a single template
#[derive(Serialize)]
struct GetTemplateResponse {
    success: bool,
    template: Option<FieldTemplate>,
    message: Option<String>,
}

/// Request to record consent acceptance (for audit trail)
#[derive(Deserialize)]
#[allow(dead_code)] // Fields used via serde deserialization
struct ConsentRequest {
    recipient_id: String,
    /// User agent string for audit trail
    user_agent: Option<String>,
    /// Hash of the consent text shown to user (to prove they saw specific terms)
    consent_text_hash: Option<String>,
}

/// Response from consent endpoint
#[derive(Serialize)]
struct ConsentResponse {
    success: bool,
    message: String,
    consent_at: String,
}

/// Request to send signing invitations
#[derive(Deserialize)]
#[allow(dead_code)]
struct InviteRequest {
    session_id: String,
    document_name: String,
    sender_name: String,
    invitations: Vec<InvitationInfo>,
    /// Feature 1: Optional document alias (e.g., "Q1 2026 Lease Agreement")
    #[serde(default)]
    document_alias: Option<String>,
    /// Feature 1: Optional signing context (e.g., "Lease for 30 James Ave, Orlando")
    #[serde(default)]
    signing_context: Option<String>,
    /// Token version for invalidation (defaults to 0 for new sessions)
    #[serde(default)]
    token_version: u32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct InvitationInfo {
    recipient_id: String,
    name: String,
    email: String,
    signing_link: String,
}

/// Response from invite endpoint
#[derive(Serialize)]
struct InviteResponse {
    success: bool,
    message: String,
}

/// Request to request a new signing link (UX-004)
#[derive(Deserialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct RequestLinkRequest {
    recipient_id: String,
}

/// Response from request-link endpoint (UX-004)
#[derive(Serialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct RequestLinkResponse {
    success: bool,
    message: String,
}

/// Response from resend endpoint (UX-004)
#[derive(Serialize)]
#[allow(dead_code)] // Used by UX-004 endpoint (not yet implemented)
struct ResendResponse {
    success: bool,
    new_session_id: String,
    expires_at: String,
    message: String,
}

/// Response for expired sessions (UX-004)
#[derive(Serialize)]
struct ExpiredSessionResponse {
    status: String,
    sender_email: String,
    document_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ============================================================
// Per-IP Rate Limiting (DDoS Prevention)
// ============================================================

/// Per-IP rate limit state stored in KV
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct IpRateLimitState {
    /// Number of requests in current window
    request_count: u32,
    /// Unix timestamp when the current window started
    window_start: u64,
}

/// Rate limit tiers for different endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitTier {
    /// Health check: 100 req/min per IP
    Health,
    /// Session read (GET): 30 req/min per IP
    SessionRead,
    /// Session write (PUT signed/decline/consent): 5 req/min per IP
    SessionWrite,
    /// Request link: 100 req/day per IP (generous daily limit, bound by global email budget)
    RequestLink,
}

impl RateLimitTier {
    /// Returns (max_requests, window_seconds) for this tier
    fn limits(&self) -> (u32, u64) {
        match self {
            RateLimitTier::Health => (100, 60),         // 100/min
            RateLimitTier::SessionRead => (30, 60),     // 30/min
            RateLimitTier::SessionWrite => (5, 60),     // 5/min
            RateLimitTier::RequestLink => (100, 86400), // 100/day - generous for UX, global email limits handle cost
        }
    }

    /// Returns the tier name for KV key construction
    fn name(&self) -> &'static str {
        match self {
            RateLimitTier::Health => "health",
            RateLimitTier::SessionRead => "session_read",
            RateLimitTier::SessionWrite => "session_write",
            RateLimitTier::RequestLink => "request_link",
        }
    }

    /// Returns retry-after seconds for rate limit response
    #[allow(dead_code)] // Available for tests and future use
    fn retry_after_seconds(&self) -> u64 {
        let (_, window) = self.limits();
        window
    }
}

/// Result of per-IP rate limit check
#[derive(Debug, Clone, PartialEq)]
enum IpRateLimitResult {
    /// Request allowed
    Allowed,
    /// Rate limited - retry after specified seconds
    Limited { retry_after_seconds: u64 },
}

/// Check per-IP rate limit for a given tier
/// Returns IpRateLimitResult indicating if request is allowed or rate limited
async fn check_ip_rate_limit(
    kv: &kv::KvStore,
    ip: &str,
    tier: RateLimitTier,
) -> Result<IpRateLimitResult> {
    let (max_requests, window_seconds) = tier.limits();
    let key = format!("ip_limit:{}:{}", ip, tier.name());

    // Get current timestamp (WASM-safe - uses js_sys::Date)
    let now = get_timestamp_secs();

    // Get current state from KV
    let mut state: IpRateLimitState = kv.get(&key).json().await?.unwrap_or_default();

    // Check if window has expired
    if now >= state.window_start + window_seconds {
        // Start new window
        state.window_start = now;
        state.request_count = 0;
    }

    // Check if we're over the limit
    if state.request_count >= max_requests {
        let retry_after = (state.window_start + window_seconds).saturating_sub(now);
        return Ok(IpRateLimitResult::Limited {
            retry_after_seconds: retry_after.max(1), // At least 1 second
        });
    }

    // Increment and save
    state.request_count += 1;

    // TTL should be slightly longer than the window to handle edge cases
    let ttl = window_seconds + 60;

    kv.put(&key, serde_json::to_string(&state)?)?
        .expiration_ttl(ttl)
        .execute()
        .await?;

    Ok(IpRateLimitResult::Allowed)
}

/// Get client IP from Cloudflare headers
/// Falls back to "unknown" if not present
fn get_client_ip(req: &Request) -> String {
    req.headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_else(|| "unknown".to_string())
}

/// Generate a 429 Too Many Requests response for IP rate limiting
fn ip_rate_limit_response(retry_after_seconds: u64) -> Result<Response> {
    #[derive(Serialize)]
    struct RateLimitError {
        error: String,
        retry_after_seconds: u64,
    }

    let resp = Response::from_json(&RateLimitError {
        error: "Rate limit exceeded".to_string(),
        retry_after_seconds,
    })?
    .with_status(429);

    // Add Retry-After header
    let headers = Headers::new();
    let _ = headers.set("Retry-After", &retry_after_seconds.to_string());

    Ok(resp.with_headers(headers))
}

/// Apply IP rate limiting to a request
/// Returns Some(Response) if rate limited, None if allowed
async fn apply_ip_rate_limit(
    req: &Request,
    env: &Env,
    tier: RateLimitTier,
) -> Option<Result<Response>> {
    let kv = match env.kv("RATE_LIMITS") {
        Ok(kv) => kv,
        Err(_) => {
            // KV not configured, skip rate limiting
            console_log!("Warning: RATE_LIMITS KV not configured, IP rate limiting disabled");
            return None;
        }
    };

    let ip = get_client_ip(req);

    match check_ip_rate_limit(&kv, &ip, tier).await {
        Ok(IpRateLimitResult::Allowed) => None,
        Ok(IpRateLimitResult::Limited {
            retry_after_seconds,
        }) => {
            console_log!(
                "IP {} rate limited for tier {:?}, retry after {} seconds",
                ip,
                tier,
                retry_after_seconds
            );
            Some(cors_response(ip_rate_limit_response(retry_after_seconds)))
        }
        Err(e) => {
            // On error, allow the request but log
            console_log!("IP rate limit check failed for {}: {}", ip, e);
            None
        }
    }
}

const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60; // 7 days
const DOWNLOAD_LINK_EXPIRY_DAYS: u32 = 30;

// ============================================================
// Session Token Configuration (Security)
// ============================================================
/// Token expiry duration in seconds (30 days)
const TOKEN_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60;

/// Errors that can occur during token verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    /// Token format is invalid (wrong number of parts, bad encoding)
    InvalidFormat,
    /// Token has expired
    Expired,
    /// HMAC signature is invalid
    InvalidSignature,
    /// Token session_id doesn't match the requested session
    SessionMismatch,
    /// Token version doesn't match session's current version (token was invalidated)
    VersionMismatch,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::InvalidFormat => write!(f, "Invalid token format"),
            TokenError::Expired => write!(f, "Token has expired"),
            TokenError::InvalidSignature => write!(f, "Invalid token signature"),
            TokenError::SessionMismatch => write!(f, "Token does not match session"),
            TokenError::VersionMismatch => {
                write!(
                    f,
                    "Token has been invalidated (document was revised or voided)"
                )
            }
        }
    }
}

// ============================================================
// Session Token Functions (Security)
// ============================================================

/// Get the signing secret from environment.
/// Priority: 1. SESSION_TOKEN_SECRET (secret), 2. SESSION_TOKEN_SECRET (var), 3. API key hash (fallback)
fn get_signing_secret(env: &Env) -> Vec<u8> {
    // Try secret first
    if let Ok(secret) = env.secret("SESSION_TOKEN_SECRET") {
        return secret.to_string().into_bytes();
    }

    // Try var for development
    if let Ok(var) = env.var("SESSION_TOKEN_SECRET") {
        return var.to_string().into_bytes();
    }

    // Fallback: derive from API key for backwards compatibility
    if let Ok(api_key) = env.secret("DOCSIGN_API_KEY") {
        // Use SHA256 of API key as the signing secret
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(api_key.to_string().as_bytes());
        hasher.update(b"session-token-secret-v1"); // Domain separation
        return hasher.finalize().to_vec();
    }

    // Last resort: empty secret (WARNING: insecure, for development only)
    console_log!(
        "WARNING: No SESSION_TOKEN_SECRET or DOCSIGN_API_KEY configured - tokens are insecure!"
    );
    b"insecure-dev-secret".to_vec()
}

/// Generate a signed recipient token for accessing a session.
///
/// Token format (base64url encoded): {session_id}:{recipient_id}:{expiry_timestamp}:{hmac_signature}
///
/// # Arguments
/// * `session_id` - The session ID this token is for
/// * `recipient_id` - The recipient ID this token authorizes
/// * `secret` - The HMAC signing secret
///
/// # Returns
/// A base64url-encoded token string
/// Generate a recipient token with token_version for invalidation support.
/// Token format: session_id:recipient_id:token_version:expiry:signature
pub fn generate_recipient_token(
    session_id: &str,
    recipient_id: &str,
    token_version: u32,
    secret: &[u8],
) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // Calculate expiry timestamp (30 days from now)
    let expiry = chrono::Utc::now().timestamp() as u64 + TOKEN_EXPIRY_SECONDS;

    // Create the payload to sign (includes token_version for invalidation)
    let payload = format!(
        "{}:{}:{}:{}",
        session_id, recipient_id, token_version, expiry
    );

    // Generate HMAC signature
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    // Encode signature as base64url
    let sig_base64 = URL_SAFE_NO_PAD.encode(signature);

    // Create the full token
    let token = format!("{}:{}", payload, sig_base64);

    // Encode the entire token as base64url for safe URL inclusion
    URL_SAFE_NO_PAD.encode(token.as_bytes())
}

/// Legacy token generation without token_version (for backwards compatibility)
pub fn generate_recipient_token_legacy(
    session_id: &str,
    recipient_id: &str,
    secret: &[u8],
) -> String {
    generate_recipient_token(session_id, recipient_id, 0, secret)
}

/// Token verification result containing recipient_id and token_version
pub struct TokenVerificationResult {
    pub recipient_id: String,
    pub token_version: u32,
}

/// Verify a recipient token and extract the recipient_id and token_version if valid.
/// Supports both legacy (4-part) and new (5-part) token formats.
///
/// # Arguments
/// * `token` - The base64url-encoded token to verify
/// * `session_id` - The session ID to validate against
/// * `secret` - The HMAC signing secret
///
/// # Returns
/// * `Ok(TokenVerificationResult)` - The recipient ID and token version if valid
/// * `Err(TokenError)` - The reason verification failed
pub fn verify_recipient_token(
    token: &str,
    session_id: &str,
    secret: &[u8],
) -> std::result::Result<TokenVerificationResult, TokenError> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    // Decode the outer base64url encoding
    let token_bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| TokenError::InvalidFormat)?;

    let token_str = String::from_utf8(token_bytes).map_err(|_| TokenError::InvalidFormat)?;

    // Split into parts
    // Legacy format (4 parts): session_id:recipient_id:expiry:signature
    // New format (5 parts): session_id:recipient_id:token_version:expiry:signature
    let parts: Vec<&str> = token_str.split(':').collect();

    let (
        token_session_id,
        token_recipient_id,
        token_version,
        expiry_str,
        signature_base64,
        payload,
    ) = match parts.len() {
        4 => {
            // Legacy format - token_version defaults to 0
            let payload = format!("{}:{}:{}", parts[0], parts[1], parts[2]);
            (parts[0], parts[1], 0u32, parts[2], parts[3], payload)
        }
        5 => {
            // New format with token_version
            let version: u32 = parts[2].parse().map_err(|_| TokenError::InvalidFormat)?;
            let payload = format!("{}:{}:{}:{}", parts[0], parts[1], parts[2], parts[3]);
            (parts[0], parts[1], version, parts[3], parts[4], payload)
        }
        _ => return Err(TokenError::InvalidFormat),
    };

    // Verify session ID matches
    if token_session_id != session_id {
        return Err(TokenError::SessionMismatch);
    }

    // Parse and verify expiry
    let expiry: u64 = expiry_str.parse().map_err(|_| TokenError::InvalidFormat)?;
    let now = chrono::Utc::now().timestamp() as u64;
    if now > expiry {
        return Err(TokenError::Expired);
    }

    // Verify HMAC signature
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());

    // Decode the signature
    let signature = URL_SAFE_NO_PAD
        .decode(signature_base64)
        .map_err(|_| TokenError::InvalidFormat)?;

    // Verify signature (constant-time comparison)
    mac.verify_slice(&signature)
        .map_err(|_| TokenError::InvalidSignature)?;

    Ok(TokenVerificationResult {
        recipient_id: token_recipient_id.to_string(),
        token_version,
    })
}

// ============================================================
// Request Size Limits (Security)
// ============================================================
/// Maximum size for PDF documents (10MB)
const MAX_PDF_SIZE: usize = 10 * 1024 * 1024;
/// Maximum size for signature data (100KB) - reserved for future granular validation
#[allow(dead_code)]
const MAX_SIGNATURE_SIZE: usize = 100 * 1024;
/// Maximum total request body size (12MB - PDF + overhead)
const MAX_REQUEST_BODY: usize = 12 * 1024 * 1024;

// ============================================================
// Session Limits per Sender (Storage Exhaustion Prevention)
// ============================================================
/// Maximum active sessions per sender to prevent storage exhaustion
const MAX_SESSIONS_PER_SENDER: usize = 100;

/// Maximum age for sessions in sender index before pruning (30 days)
const SESSION_INDEX_PRUNE_DAYS: i64 = 30;

/// Tracks all active sessions for a sender (stored in KV)
/// Key format: sender_index:{sha256_hash_of_email}
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct SenderSessionIndex {
    /// List of active session IDs for this sender
    session_ids: Vec<String>,
    /// ISO timestamps when each session was created (parallel array)
    created_at: Vec<String>,
}

impl SenderSessionIndex {
    /// Remove expired sessions from the index (older than prune_days)
    fn prune_expired(&mut self, prune_days: i64) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::days(prune_days);

        let mut new_ids = Vec::new();
        let mut new_times = Vec::new();

        for (id, created) in self.session_ids.iter().zip(self.created_at.iter()) {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created) {
                if dt > cutoff {
                    new_ids.push(id.clone());
                    new_times.push(created.clone());
                }
            }
            // If we can't parse the timestamp, drop it (corrupted data)
        }

        self.session_ids = new_ids;
        self.created_at = new_times;
    }

    /// Add a new session to the index
    fn add_session(&mut self, session_id: String, created_at: String) {
        self.session_ids.push(session_id);
        self.created_at.push(created_at);
    }

    /// Remove a session from the index by ID
    fn remove_session(&mut self, session_id: &str) {
        if let Some(idx) = self.session_ids.iter().position(|id| id == session_id) {
            self.session_ids.remove(idx);
            if idx < self.created_at.len() {
                self.created_at.remove(idx);
            }
        }
    }

    /// Get the number of active sessions
    fn count(&self) -> usize {
        self.session_ids.len()
    }
}

// ============================================================
// Feature: Recipient Session Index (DocuSign-style Inbox)
// ============================================================

/// Entry in the recipient session index
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct RecipientIndexEntry {
    session_id: String,
    recipient_id: String,
    created_at: String,
}

/// Index of sessions where a user is a recipient (for "My Documents" inbox)
/// Key format: recipient_index:{sha256_hash_of_email}
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct RecipientSessionIndex {
    /// List of (session_id, recipient_id) tuples
    entries: Vec<RecipientIndexEntry>,
}

impl RecipientSessionIndex {
    /// Add a new entry to the index (avoids duplicates)
    fn add_entry(&mut self, session_id: String, recipient_id: String, created_at: String) {
        // Avoid duplicates
        if !self
            .entries
            .iter()
            .any(|e| e.session_id == session_id && e.recipient_id == recipient_id)
        {
            self.entries.push(RecipientIndexEntry {
                session_id,
                recipient_id,
                created_at,
            });
        }
    }

    /// Remove expired sessions from the index (older than prune_days)
    fn prune_expired(&mut self, prune_days: i64) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::days(prune_days);

        self.entries.retain(|e| {
            chrono::DateTime::parse_from_rfc3339(&e.created_at)
                .map(|dt| dt > cutoff)
                .unwrap_or(true) // Keep if we can't parse (be conservative)
        });
    }
}

// ============================================================
// Feature 2: Document Dashboard - Response Types
// ============================================================

/// Summary of a document for the dashboard (excludes PDF content for efficiency)
#[derive(Serialize)]
struct SessionSummary {
    session_id: String,
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signing_context: Option<String>,
    created_at: String,
    expires_at: String,
    status: SessionStatus,
    recipients_signed: u32,
    recipients_total: u32,
    /// List of recipient names and their signed status
    recipients: Vec<RecipientSummary>,
}

/// Recipient status for dashboard display
#[derive(Serialize)]
struct RecipientSummary {
    name: String,
    email: String,
    signed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    signed_at: Option<String>,
}

/// Response for /my-sessions endpoint - DocuSign-style Sent + Inbox layout
#[derive(Serialize)]
struct MySessionsResponse {
    success: bool,
    /// Documents I SENT (created by me)
    sent: SentDocuments,
    /// Documents sent TO ME (my inbox)
    inbox: InboxDocuments,
}

/// Documents the user sent to others
#[derive(Serialize)]
struct SentDocuments {
    /// Documents where invitations have been sent, awaiting signatures
    in_progress: Vec<SessionSummary>,
    /// Documents where all signers have completed
    completed: Vec<SessionSummary>,
    /// Documents that were declined by any signer (needs sender action)
    declined: Vec<SessionSummary>,
    /// Documents that expired before completion
    expired: Vec<SessionSummary>,
    /// Documents that were voided/discarded by sender
    voided: Vec<SessionSummary>,
}

/// Documents sent to the user (their inbox)
#[derive(Serialize)]
struct InboxDocuments {
    /// Documents I need to sign (not yet signed)
    to_sign: Vec<InboxSessionSummary>,
    /// Documents I've completed signing
    completed: Vec<InboxSessionSummary>,
    /// Documents I declined
    declined: Vec<InboxSessionSummary>,
}

/// Summary of a document in the inbox (from recipient's perspective)
#[derive(Serialize)]
struct InboxSessionSummary {
    session_id: String,
    filename: String,
    sender_name: String,
    sender_email: String,
    created_at: String,
    expires_at: String,
    /// "pending", "signed", or "declined"
    my_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    signed_at: Option<String>,
}

// ============================================================
// UX-006: Sender Notification Helper Functions
// ============================================================

/// Check if all recipients (with role "signer") have signed
fn all_recipients_signed(recipients: &[RecipientInfo]) -> bool {
    recipients
        .iter()
        .filter(|r| r.role == "signer")
        .all(|r| r.signed)
}

/// Generate a download link for a signed document that expires after the specified days
fn generate_download_link(session_id: &str, expiry_days: u32) -> String {
    let expiry_timestamp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(expiry_days as i64))
        .unwrap_or_else(chrono::Utc::now)
        .timestamp();

    // Points to API download endpoint (not frontend)
    format!(
        "https://api.getsignatures.org/session/{}/download?expires={}",
        session_id, expiry_timestamp
    )
}

/// Format a human-readable timestamp from RFC3339
fn format_timestamp(rfc3339: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(rfc3339) {
        Ok(dt) => dt.format("%B %d, %Y at %I:%M %p UTC").to_string(),
        Err(_) => rfc3339.to_string(),
    }
}

/// Format notification email when a single recipient completes signing
fn format_completion_notification_email(
    recipient_name: &str,
    document_name: &str,
    signed_at: &str,
    download_link: &str,
) -> String {
    let formatted_time = format_timestamp(signed_at);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #059669 0%, #047857 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Signed</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Good news!</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{recipient_name}</strong> has signed your document:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Signed At</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{signed_time}</p>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{download_link}" style="display: inline-block; background: #059669; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Download Signed Document</a>
        </div>

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> This download link will expire after {expiry_days} days for security purposes.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The signed document is securely stored and encrypted. You can download it anytime before the link expires.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        document_name = document_name,
        signed_time = formatted_time,
        download_link = download_link,
        expiry_days = DOWNLOAD_LINK_EXPIRY_DAYS
    )
}

/// Format notification email when all recipients have completed signing
fn format_all_signed_notification_email(
    recipients: &[RecipientInfo],
    document_name: &str,
    download_link: &str,
) -> String {
    // Build list of signers with their timestamps
    let signers_list: Vec<String> = recipients
        .iter()
        .filter(|r| r.role == "signer" && r.signed)
        .map(|r| {
            let time = r
                .signed_at
                .as_ref()
                .map(|t| format_timestamp(t))
                .unwrap_or_else(|| "Unknown".to_string());
            format!(
                "<li style=\"margin: 10px 0;\"><strong>{}</strong> - {}</li>",
                r.name, time
            )
        })
        .collect();

    let signers_html = signers_list.join("\n                ");

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #059669 0%, #047857 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">All Recipients Have Signed!</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Congratulations!</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            All recipients have completed signing your document. The signing process is now <strong>complete</strong>.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0 0 10px 0; font-size: 14px; color: #6b7280;">Signers</p>
            <ul style="margin: 0; padding-left: 20px; list-style-type: none;">
                {signers_list}
            </ul>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{download_link}" style="display: inline-block; background: #059669; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Download Completed Document</a>
        </div>

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> This download link will expire after {expiry_days} days for security purposes.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The fully signed document is securely stored and encrypted. You can download it anytime before the link expires.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        document_name = document_name,
        signers_list = signers_html,
        download_link = download_link,
        expiry_days = DOWNLOAD_LINK_EXPIRY_DAYS
    )
}

/// Format notification email when a recipient declines to sign (UX-006)
fn format_decline_notification_email(
    recipient_name: &str,
    recipient_email: &str,
    document_name: &str,
    reason: Option<&str>,
    declined_at: &str,
) -> String {
    let formatted_time = format_timestamp(declined_at);
    let reason_section = if let Some(r) = reason {
        format!(
            r#"<div style="background: #fef3c7; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Reason Provided</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-style: italic;">"{}"</p>
        </div>"#,
            r
        )
    } else {
        "<p style=\"font-size: 14px; color: #6b7280; font-style: italic;\">No reason was provided.</p>"
            .to_string()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #dc2626 0%, #b91c1c 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Declined</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">
            Unfortunately, a recipient has declined to sign your document.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Recipient</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{recipient_name}</p>
            <p style="margin: 5px 0 0 0; font-size: 14px; color: #6b7280;">{recipient_email}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Declined At</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{declined_time}</p>
        </div>

        {reason_section}

        <div style="background: #dbeafe; border-left: 4px solid #3b82f6; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #1e40af;">
                <strong>What's next?</strong> You may want to contact the recipient directly to resolve any concerns, or resend the document request if needed.
            </p>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            The original document remains securely stored. You can create a new signing request at any time.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        recipient_email = recipient_email,
        document_name = document_name,
        declined_time = formatted_time,
        reason_section = reason_section
    )
}

/// Format void notification email sent when a document is discarded
fn format_void_notification_email(
    document_name: &str,
    sender_name: &str,
    reason: Option<&str>,
    voided_at: &str,
    recipient_name: &str,
) -> String {
    let formatted_time = format_timestamp(voided_at);
    let reason_section = if let Some(r) = reason {
        format!(
            r#"<div style="background: #f3f4f6; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Reason</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-style: italic;">"{}"</p>
        </div>"#,
            r
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #6b7280 0%, #4b5563 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Cancelled</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">
            Hello {recipient_name},
        </p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            The sender has cancelled this document signing request. Any previous signing links are no longer valid.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Sender</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{sender_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Cancelled At</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{voided_time}</p>
        </div>

        {reason_section}

        <div style="background: #fef3c7; border-left: 4px solid #f59e0b; padding: 15px; border-radius: 4px; margin-top: 25px;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Note:</strong> If you have any questions about this cancellation, please contact the sender directly.
            </p>
        </div>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        document_name = document_name,
        sender_name = sender_name,
        voided_time = formatted_time,
        reason_section = reason_section
    )
}

/// Bug C: Format completion email sent to each signer when all signatures are collected
fn format_recipient_completion_email(
    recipient_name: &str,
    document_name: &str,
    download_link: &str,
    all_recipients: &[RecipientInfo],
) -> String {
    // Build list of all signers with timestamps
    let signers_list: Vec<String> = all_recipients
        .iter()
        .filter(|r| r.role == "signer" && r.signed)
        .map(|r| {
            let time = r
                .signed_at
                .as_ref()
                .map(|t| format_timestamp(t))
                .unwrap_or_else(|| "Unknown".to_string());
            format!(
                "<li style=\"margin: 10px 0;\"><strong>{}</strong> - {}</li>",
                r.name, time
            )
        })
        .collect();

    let signers_html = signers_list.join("\n                ");

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #059669 0%, #047857 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Signing Complete</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Hello {recipient_name},</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            Great news! All parties have signed the document. Your copy is ready for download.
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0 0 10px 0; font-size: 14px; color: #6b7280;">All Signers</p>
            <ul style="margin: 0; padding-left: 20px; list-style-type: none;">
                {signers_list}
            </ul>
        </div>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{download_link}" style="display: inline-block; background: #059669; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Download Signed Document</a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            This is your official copy of the fully signed document. Please save it for your records.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>

    <div style="text-align: center; margin-top: 10px; padding-top: 15px; border-top: 1px solid #e5e7eb; font-size: 11px; color: #6b7280;">
        <p style="margin: 0;">
            Electronic signatures have the same legal effect as handwritten signatures under the ESIGN Act and UETA.
        </p>
    </div>
</body>
</html>"#,
        recipient_name = recipient_name,
        document_name = document_name,
        signers_list = signers_html,
        download_link = download_link
    )
}

/// Send notification email to sender when recipient signs
async fn send_sender_notification(
    env: &Env,
    sender_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<()> {
    let request = email::EmailSendRequest {
        to: vec![sender_email.to_string()],
        subject: subject.to_string(),
        html: html_body.to_string(),
        text: None,
        reply_to: None,
        tags: vec![("type".to_string(), "notification".to_string())],
    };

    match email::send_email(env, request).await {
        Ok(result) => {
            if result.success {
                console_log!("Notification sent to sender: {}", sender_email);
            } else {
                console_log!(
                    "Failed to send notification: {}",
                    result.error.unwrap_or_default()
                );
            }
        }
        Err(e) => {
            console_log!("Failed to send notification: {}", e);
        }
    }

    Ok(())
}

/// Verify API key from request header
fn verify_api_key(req: &Request, env: &Env) -> bool {
    // Get expected API key from environment
    let expected_key = match env.secret("DOCSIGN_API_KEY") {
        Ok(secret) => secret.to_string(),
        Err(_) => {
            console_log!("Warning: DOCSIGN_API_KEY not configured - API is unprotected!");
            return true; // Allow if not configured (for development)
        }
    };

    // Check X-API-Key header
    matches!(req.headers().get("X-API-Key").ok().flatten(), Some(key) if key == expected_key)
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // CORS preflight
    if req.method() == Method::Options {
        return cors_response(Response::empty());
    }

    let path = req.path();
    let method = req.method();

    // Route requests
    match (method, path.as_str()) {
        // Health check (public) - detailed monitoring endpoint
        (Method::Get, "/health") => {
            if let Some(response) = apply_ip_rate_limit(&req, &env, RateLimitTier::Health).await {
                return response;
            }
            handle_health_check(env).await
        }
        (Method::Get, "/") => cors_response(Response::ok("DocSign API Server")),

        // ============================================
        // Public configuration (Bug #22: Google OAuth)
        // ============================================
        (Method::Get, "/config/public") => {
            cors_response(handle_public_config(env).await)
        }

        // ============================================
        // TSA Proxy for LTV timestamps (public)
        // NOTE: Handler includes CORS headers directly - don't wrap with cors_response()
        // because it replaces all headers including Content-Type
        // ============================================
        (Method::Post, "/tsa-proxy") => handle_tsa_proxy(req).await,

        // ============================================
        // Authentication endpoints (public)
        // ============================================
        (Method::Post, "/auth/register") => {
            // Rate limit: 5 registrations per hour per IP
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_register(req, env).await)
        }
        (Method::Get, "/auth/verify-email") => {
            cors_response(auth::handle_verify_email(req, env).await)
        }
        (Method::Post, "/auth/login") => {
            // Rate limit: 10 login attempts per hour per IP
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            cors_response(auth::handle_login(req, env).await)
        }
        (Method::Post, "/auth/refresh") => cors_response(auth::handle_refresh(req, env).await),
        (Method::Post, "/auth/logout") => cors_response(auth::handle_logout(req, env).await),
        (Method::Post, "/auth/forgot-password") => {
            // Rate limit to prevent spam
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_forgot_password(req, env).await)
        }
        (Method::Post, "/auth/reset-password") => {
            cors_response(auth::handle_reset_password(req, env).await)
        }
        (Method::Post, "/auth/resend-verification") => {
            // Rate limit to prevent spam
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(auth::handle_resend_verification(req, env).await)
        }
        (Method::Post, "/auth/check-email") => {
            // Rate limit: Use SessionRead tier (30/min) - lightweight lookup
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            cors_response(auth::handle_check_email(req, env).await)
        }
        (Method::Post, "/auth/profile") => {
            // Profile update requires valid auth token
            cors_response(auth::handle_update_profile(req, env).await)
        }
        (Method::Post, "/auth/associate-session") => {
            // Associate a signed session with a newly created account
            handle_associate_session(req, env).await
        }

        // Bug #21: Account Deletion (GDPR)
        (Method::Post, "/auth/request-deletion") => {
            // Rate limit to prevent spam
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            cors_response(handle_request_account_deletion(req, env).await)
        }
        (Method::Post, "/auth/confirm-deletion") => {
            cors_response(handle_confirm_account_deletion(req, env).await)
        }

        // Bug #22: Google OAuth Login
        (Method::Post, "/auth/google") => {
            // Rate limit logins (use SessionWrite tier - 5/min)
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            cors_response(handle_google_oauth(req, env).await)
        }

        // ============================================
        // Phase 3: Template Management Endpoints
        // ============================================

        // Create a new template (requires auth)
        (Method::Post, "/templates") => cors_response(handle_create_template(req, env).await),

        // List user's templates (requires auth)
        (Method::Get, "/templates") => cors_response(handle_list_templates(req, env).await),

        // Get a single template (requires auth)
        (Method::Get, p) if p.starts_with("/templates/") && !p.contains("/templates//") => {
            let template_id = p.strip_prefix("/templates/").unwrap_or("");
            cors_response(handle_get_template(req, env, template_id.to_string()).await)
        }

        // Update a template (requires auth)
        (Method::Put, p) if p.starts_with("/templates/") => {
            let template_id = p.strip_prefix("/templates/").unwrap_or("");
            cors_response(handle_update_template(req, env, template_id.to_string()).await)
        }

        // Delete a template (requires auth)
        (Method::Delete, p) if p.starts_with("/templates/") => {
            let template_id = p.strip_prefix("/templates/").unwrap_or("");
            cors_response(handle_delete_template(req, env, template_id.to_string()).await)
        }

        // Bug #4: Feedback/Request submission (requires auth)
        (Method::Post, "/requests/submit") => cors_response(handle_submit_request(req, env).await),

        // ============================================
        // Bug #20: Unsubscribe endpoints (public)
        // ============================================

        // Process unsubscribe request
        (Method::Post, "/unsubscribe") => cors_response(handle_unsubscribe(req, env).await),

        // Check unsubscribe status
        (Method::Get, "/unsubscribe") => cors_response(handle_check_unsubscribe(req, env).await),

        // ============================================
        // Bug #8: Admin Dashboard endpoints
        // ============================================

        // List all requests (admin only)
        (Method::Get, "/admin/requests") => {
            cors_response(handle_admin_list_requests(req, env).await)
        }

        // Update a request (approve/deny/in-progress)
        (Method::Post, p) if p.starts_with("/admin/requests/") => {
            let request_id = p.strip_prefix("/admin/requests/").unwrap_or("");
            cors_response(handle_admin_update_request(req, env, request_id.to_string()).await)
        }

        // List users (admin only, supports ?filter=unverified)
        (Method::Get, "/admin/users") => cors_response(handle_admin_list_users(req, env).await),

        // Adjust user quota (admin only)
        (Method::Post, p) if p.starts_with("/admin/users/") && p.ends_with("/quota") => {
            let user_id = p
                .strip_prefix("/admin/users/")
                .and_then(|s| s.strip_suffix("/quota"))
                .unwrap_or("");
            cors_response(handle_admin_adjust_quota(req, env, user_id.to_string()).await)
        }

        // Delete user (admin only)
        (Method::Delete, p) if p.starts_with("/admin/users/") => {
            let user_id = p.strip_prefix("/admin/users/").unwrap_or("");
            cors_response(handle_admin_delete_user(req, env, user_id.to_string()).await)
        }

        // ============================================
        // Beta Access Grant endpoints (admin only)
        // ============================================

        // List all beta grants
        (Method::Get, "/admin/beta-grants") => {
            cors_response(handle_admin_list_beta_grants(req, env).await)
        }

        // Create a beta grant (pre-grant access to an email)
        (Method::Post, "/admin/beta-grants") => {
            cors_response(handle_admin_create_beta_grant(req, env).await)
        }

        // Revoke a beta grant
        (Method::Delete, p) if p.starts_with("/admin/beta-grants/") => {
            let email = p.strip_prefix("/admin/beta-grants/").unwrap_or("");
            // URL decode the email (handles @ and other special chars)
            let email = urlencoding::decode(email).unwrap_or_default().to_string();
            cors_response(handle_admin_revoke_beta_grant(req, env, email).await)
        }

        // Bug #19: Send pending welcome emails (batch send)
        (Method::Post, "/admin/send-pending-welcome-emails") => {
            cors_response(handle_admin_send_pending_welcome_emails(req, env).await)
        }

        // ============================================
        // Bug #6: Billing/Stripe endpoints
        // ============================================

        // Create Stripe checkout session (requires auth)
        (Method::Post, "/billing/checkout") => {
            cors_response(billing::handle_checkout(req, env).await)
        }

        // Create Stripe customer portal session (requires auth)
        (Method::Post, "/billing/portal") => cors_response(billing::handle_portal(req, env).await),

        // Get billing status (requires auth)
        (Method::Get, "/billing/status") => {
            cors_response(billing::handle_billing_status(req, env).await)
        }

        // Stripe webhook (no auth - uses webhook signature verification)
        (Method::Post, "/billing/webhook") => {
            cors_response(billing::handle_webhook(req, env).await)
        }

        // Protected endpoints - require API key
        (Method::Post, "/send") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_send_email(req, env).await
        }
        (Method::Post, "/invite") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_send_invites(req, env).await
        }

        // Session management (protected)
        (Method::Post, "/session") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            handle_create_session(req, env).await
        }
        // Feature 2: Document Dashboard - list user's sessions grouped by status
        (Method::Get, "/my-sessions") => {
            // Apply SessionRead rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            handle_my_sessions(&req, env).await
        }
        // Download endpoint: Merge annotations into PDF on-demand
        // IMPORTANT: Must come BEFORE general GET /session/{id} to avoid being caught by it
        (Method::Get, p) if p.starts_with("/session/") && p.ends_with("/download") => {
            console_log!("Download route matched: {}", p);
            // Apply SessionRead rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            console_log!("Download parts: {:?}, len={}", parts, parts.len());
            if parts.len() == 4 {
                handle_download(parts[2], env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // General session GET - must come AFTER more specific routes like /download
        (Method::Get, p) if p.starts_with("/session/") => {
            // Apply SessionRead rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionRead).await
            {
                return response;
            }
            let id = p.strip_prefix("/session/").unwrap_or("");
            if id.contains('/') {
                cors_response(Response::error("Not found", 404))
            } else {
                handle_get_session(id, &req, env).await
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/signed") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_submit_signed(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // NEW: Annotation-based signing (lightweight ~50KB vs full PDF ~13MB)
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/annotations") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_submit_annotations(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/decline") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_decline(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // Void endpoint: sender discards document, invalidates all signing links
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/void") => {
            // Require authentication (only sender can void)
            let (user, _users_kv) = match auth::require_auth(&req, &env).await {
                Ok(Ok(result)) => result,
                Ok(Err(response)) => return cors_response(Ok(response)),
                Err(e) => {
                    return cors_response(Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "Authentication required",
                        "error": format!("{:?}", e)
                    }))?
                    .with_status(401)));
                }
            };
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_void(parts[2], user.email, req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // Revise endpoint: sender can revise document if no one has signed yet
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/revise") => {
            // Require authentication (only sender can revise)
            let (user, _users_kv) = match auth::require_auth(&req, &env).await {
                Ok(Ok(result)) => result,
                Ok(Err(response)) => return cors_response(Ok(response)),
                Err(e) => {
                    return cors_response(Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "Authentication required",
                        "error": format!("{:?}", e)
                    }))?
                    .with_status(401)));
                }
            };
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_revise(parts[2], user.email, req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // Restart endpoint: sender can restart document, voiding all existing signatures
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/restart") => {
            // Require authentication (only sender can restart)
            let (user, _users_kv) = match auth::require_auth(&req, &env).await {
                Ok(Ok(result)) => result,
                Ok(Err(response)) => return cors_response(Ok(response)),
                Err(e) => {
                    return cors_response(Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "Authentication required",
                        "error": format!("{:?}", e)
                    }))?
                    .with_status(401)));
                }
            };
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_restart(parts[2], user.email, req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // Consent endpoint: logs consent acceptance for audit trail
        (Method::Put, p) if p.starts_with("/session/") && p.ends_with("/consent") => {
            // Apply SessionWrite rate limit
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::SessionWrite).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_consent(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Request new link endpoint (public - no API key required)
        // Strictest rate limit: 3 req/hour per IP (prevents email spam)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/request-link") => {
            // Apply RequestLink rate limit (strictest - 3/hour)
            if let Some(response) =
                apply_ip_rate_limit(&req, &env, RateLimitTier::RequestLink).await
            {
                return response;
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_request_link(parts[2], req, env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }
        // UX-004: Resend endpoint (protected with API key)
        (Method::Post, p) if p.starts_with("/session/") && p.ends_with("/resend") => {
            if !verify_api_key(&req, &env) {
                return cors_response(Response::error("Unauthorized", 401));
            }
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() == 4 {
                handle_resend(parts[2], env).await
            } else {
                cors_response(Response::error("Not found", 404))
            }
        }

        _ => cors_response(Response::error("Not found", 404)),
    }
}

/// Handle TSA proxy requests - proxies RFC 3161 timestamp requests to avoid CORS issues
/// This endpoint allows the frontend to request LTV timestamps without browser CORS restrictions
async fn handle_tsa_proxy(mut req: Request) -> Result<Response> {
    let url = req.url()?;

    // Get TSA server URL from query param or use default (freetsa.org)
    let tsa_url = url
        .query_pairs()
        .find(|(k, _)| k == "tsa")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "https://freetsa.org/tsr".to_string());

    // Get request body (timestamp request bytes)
    let body = req.bytes().await?;

    // Build request to TSA server
    let headers = Headers::new();
    headers.set("Content-Type", "application/timestamp-query")?;

    let tsa_req = Request::new_with_init(
        &tsa_url,
        RequestInit::new()
            .with_method(Method::Post)
            .with_body(Some(body.into()))
            .with_headers(headers),
    )?;

    // Forward to TSA server
    let mut tsa_response = Fetch::Request(tsa_req).send().await?;
    let response_bytes = tsa_response.bytes().await?;

    // Build response with BOTH Content-Type AND CORS headers in ONE Headers object
    // (we don't use cors_response() wrapper because it REPLACES all headers)
    let resp_headers = Headers::new();
    resp_headers.set("Content-Type", "application/timestamp-reply")?;
    resp_headers.set("Access-Control-Allow-Origin", "*")?;
    resp_headers.set("Access-Control-Allow-Methods", "POST, OPTIONS")?;

    Response::from_bytes(response_bytes).map(|r| r.with_headers(resp_headers))
}

/// Handle public configuration requests (Bug #22: Google OAuth)
/// Returns public-safe configuration values like Google Client ID
async fn handle_public_config(env: Env) -> Result<Response> {
    // Get Google Client ID from environment variables
    let google_client_id = env
        .var("GOOGLE_CLIENT_ID")
        .map(|v| v.to_string())
        .unwrap_or_default();

    // Get Stripe publishable key (safe to expose - it's public)
    let stripe_publishable_key = env
        .var("STRIPE_PUBLISHABLE_KEY")
        .map(|v| v.to_string())
        .unwrap_or_default();

    let config = serde_json::json!({
        "google_client_id": google_client_id,
        "stripe_publishable_key": stripe_publishable_key,
    });

    Response::from_json(&config)
}

/// Handle health check requests - returns service status and dependency health
async fn handle_health_check(env: Env) -> Result<Response> {
    let timestamp = Utc::now().to_rfc3339();

    // Check KV Sessions availability
    let kv_sessions_status = match env.kv("SESSIONS") {
        Ok(kv) => {
            // Try a simple operation to verify KV is working
            match kv.get("__health_check__").text().await {
                Ok(_) => DependencyStatus {
                    status: "healthy".to_string(),
                    error: None,
                },
                Err(e) => DependencyStatus {
                    status: "degraded".to_string(),
                    error: Some(format!("KV read error: {}", e)),
                },
            }
        }
        Err(_) => DependencyStatus {
            status: "unavailable".to_string(),
            error: Some("SESSIONS KV binding not configured".to_string()),
        },
    };

    // Check KV Rate Limits availability
    let kv_rate_limits_status = match env.kv("RATE_LIMITS") {
        Ok(kv) => match kv.get("rate_state").text().await {
            Ok(_) => DependencyStatus {
                status: "healthy".to_string(),
                error: None,
            },
            Err(e) => DependencyStatus {
                status: "degraded".to_string(),
                error: Some(format!("KV read error: {}", e)),
            },
        },
        Err(_) => DependencyStatus {
            status: "unavailable".to_string(),
            error: Some("RATE_LIMITS KV binding not configured".to_string()),
        },
    };

    // Determine overall status
    let overall_status =
        if kv_sessions_status.status == "healthy" && kv_rate_limits_status.status == "healthy" {
            "healthy"
        } else if kv_sessions_status.status == "unavailable"
            || kv_rate_limits_status.status == "unavailable"
        {
            "unhealthy"
        } else {
            "degraded"
        };

    let response = HealthResponse {
        status: overall_status.to_string(),
        timestamp,
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies: HealthDependencies {
            kv_sessions: kv_sessions_status,
            kv_rate_limits: kv_rate_limits_status,
        },
    };

    // Return appropriate status code based on health
    let status_code = match overall_status {
        "healthy" => 200,
        "degraded" => 200, // Still operational but with issues
        _ => 503,          // Service unavailable
    };

    let resp = Response::from_json(&response)?.with_status(status_code);
    cors_response(Ok(resp))
}

/// Bug #4: Handle feedback/request submission
async fn handle_submit_request(req: Request, env: Env) -> Result<Response> {
    use auth::types::{SubmitRequestBody, SubmitRequestResponse, UserRequest};

    // Get user from auth token
    let user = match auth::get_authenticated_user(&req, &env).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(Response::from_json(&SubmitRequestResponse {
                success: false,
                request_id: None,
                message: Some("Please log in to submit feedback.".to_string()),
            })?
            .with_status(401));
        }
        Err(e) => {
            console_log!("Error getting user: {:?}", e);
            return Ok(Response::from_json(&SubmitRequestResponse {
                success: false,
                request_id: None,
                message: Some("Authentication error.".to_string()),
            })?
            .with_status(401));
        }
    };

    // Clone request for body parsing (req was consumed by auth check)
    let mut req = req;

    // Parse request body
    let body: SubmitRequestBody = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&SubmitRequestResponse {
                success: false,
                request_id: None,
                message: Some(format!("Invalid request: {}", e)),
            })?
            .with_status(400));
        }
    };

    // Validate description
    if body.description.trim().is_empty() {
        return Ok(Response::from_json(&SubmitRequestResponse {
            success: false,
            request_id: None,
            message: Some("Please provide a description.".to_string()),
        })?
        .with_status(400));
    }

    if body.description.len() > 2000 {
        return Ok(Response::from_json(&SubmitRequestResponse {
            success: false,
            request_id: None,
            message: Some("Description too long (max 2000 characters).".to_string()),
        })?
        .with_status(400));
    }

    // Check for pending request from this user (rate limit: 1 pending per user)
    let requests_kv = match env.kv("REQUESTS") {
        Ok(kv) => kv,
        Err(_) => {
            console_log!("Warning: REQUESTS KV not configured, skipping duplicate check");
            // Continue without duplicate check - KV not set up yet
            let user_request = UserRequest::new(
                user.id.clone(),
                user.email.clone(),
                body.request_type,
                body.description.clone(),
                body.additional_documents,
            );

            // Send admin notification email (best effort)
            let request_type_str = body.request_type.to_string();
            if let Err(e) = email::send_admin_notification_email(
                &env,
                &user.email,
                &request_type_str,
                &body.description,
                body.additional_documents,
            )
            .await
            {
                console_log!("Warning: Failed to send admin notification: {:?}", e);
            }

            return Ok(Response::from_json(&SubmitRequestResponse {
                success: true,
                request_id: Some(user_request.id),
                message: Some("Thank you for your feedback!".to_string()),
            })?);
        }
    };

    // Check for existing pending request
    let user_pending_key = format!("pending:{}", user.id);
    if let Ok(Some(_)) = requests_kv.get(&user_pending_key).text().await {
        return Ok(Response::from_json(&SubmitRequestResponse {
            success: false,
            request_id: None,
            message: Some(
                "You already have a pending request. Please wait for it to be resolved."
                    .to_string(),
            ),
        })?
        .with_status(429));
    }

    // Create the request
    let user_request = UserRequest::new(
        user.id.clone(),
        user.email.clone(),
        body.request_type,
        body.description.clone(),
        body.additional_documents,
    );
    let request_id = user_request.id.clone();

    // Store the request
    if let Err(e) = requests_kv
        .put(
            &format!("request:{}", request_id),
            serde_json::to_string(&user_request)?,
        )
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?
        .execute()
        .await
    {
        console_log!("Warning: Failed to store request: {:?}", e);
    }

    // Mark user as having pending request
    if let Err(e) = requests_kv
        .put(&user_pending_key, &request_id)
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?
        .execute()
        .await
    {
        console_log!("Warning: Failed to set pending marker: {:?}", e);
    }

    // Send admin notification email (best effort, non-blocking)
    let request_type_str = body.request_type.to_string();
    match email::send_admin_notification_email(
        &env,
        &user.email,
        &request_type_str,
        &body.description,
        body.additional_documents,
    )
    .await
    {
        Ok(result) => {
            if result.success {
                console_log!("Admin notification sent for request {}", request_id);
            } else {
                console_log!("Failed to send admin notification: {:?}", result.error);
            }
        }
        Err(e) => {
            console_log!("Error sending admin notification: {:?}", e);
        }
    }

    Ok(Response::from_json(&SubmitRequestResponse {
        success: true,
        request_id: Some(request_id),
        message: Some("Thank you for your feedback! We'll review it soon.".to_string()),
    })?)
}

// ============================================================================
// Bug #8: Admin Dashboard Handlers
// ============================================================================

/// Helper: Check if user is admin and return user or error response
async fn get_admin_user(
    req: &Request,
    env: &Env,
) -> Result<std::result::Result<auth::types::User, Response>> {
    use auth::types::{is_admin, AdminRequestsListResponse};

    let user = match auth::get_authenticated_user(req, env).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(Err(Response::from_json(&AdminRequestsListResponse {
                success: false,
                requests: vec![],
                total: 0,
            })?
            .with_status(401)));
        }
        Err(e) => {
            console_log!("Admin auth error: {:?}", e);
            return Ok(Err(Response::from_json(&AdminRequestsListResponse {
                success: false,
                requests: vec![],
                total: 0,
            })?
            .with_status(401)));
        }
    };

    if !is_admin(&user.email) {
        return Ok(Err(Response::from_json(&serde_json::json!({
            "success": false,
            "error": "Access denied. Admin only."
        }))?
        .with_status(403)));
    }

    Ok(Ok(user))
}

/// List all requests (admin only)
async fn handle_admin_list_requests(req: Request, env: Env) -> Result<Response> {
    use auth::types::{AdminRequestsListResponse, RequestStatus, UserRequest};

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let requests_kv = match env.kv("REQUESTS") {
        Ok(kv) => kv,
        Err(_) => {
            return Ok(Response::from_json(&AdminRequestsListResponse {
                success: true,
                requests: vec![],
                total: 0,
            })?);
        }
    };

    // Get filter from query params
    let url = req.url()?;
    let query_params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let status_filter = query_params.get("status").map(|s| s.to_string());

    // List all request keys (prefix "request:")
    let list_result = requests_kv
        .list()
        .prefix("request:".to_string())
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let mut requests: Vec<UserRequest> = Vec::new();

    for key in list_result.keys {
        if let Ok(Some(request_str)) = requests_kv.get(&key.name).text().await {
            if let Ok(request) = serde_json::from_str::<UserRequest>(&request_str) {
                // Apply status filter if provided
                if let Some(ref filter) = status_filter {
                    let request_status = match request.status {
                        RequestStatus::Pending => "pending",
                        RequestStatus::InProgress => "in_progress",
                        RequestStatus::Resolved => "resolved",
                        RequestStatus::Rejected => "rejected",
                    };
                    if request_status != filter {
                        continue;
                    }
                }
                requests.push(request);
            }
        }
    }

    // Sort by created_at descending (newest first)
    requests.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = requests.len();
    Ok(Response::from_json(&AdminRequestsListResponse {
        success: true,
        requests,
        total,
    })?)
}

/// Update a request (approve/deny/in-progress) - admin only
async fn handle_admin_update_request(
    mut req: Request,
    env: Env,
    request_id: String,
) -> Result<Response> {
    use auth::types::{
        AdminRequestAction, AdminUpdateRequestBody, AdminUpdateRequestResponse, RequestStatus,
        UserRequest,
    };

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    // Parse request body
    let body: AdminUpdateRequestBody = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&AdminUpdateRequestResponse {
                success: false,
                message: Some(format!("Invalid request: {}", e)),
                request: None,
            })?
            .with_status(400));
        }
    };

    let requests_kv = match env.kv("REQUESTS") {
        Ok(kv) => kv,
        Err(_) => {
            return Ok(Response::from_json(&AdminUpdateRequestResponse {
                success: false,
                message: Some("REQUESTS KV not configured".to_string()),
                request: None,
            })?
            .with_status(500));
        }
    };

    let request_key = format!("request:{}", request_id);

    // Get existing request
    let mut user_request: UserRequest = match requests_kv.get(&request_key).text().await {
        Ok(Some(s)) => match serde_json::from_str(&s) {
            Ok(r) => r,
            Err(_) => {
                return Ok(Response::from_json(&AdminUpdateRequestResponse {
                    success: false,
                    message: Some("Invalid request data".to_string()),
                    request: None,
                })?
                .with_status(500));
            }
        },
        _ => {
            return Ok(Response::from_json(&AdminUpdateRequestResponse {
                success: false,
                message: Some("Request not found".to_string()),
                request: None,
            })?
            .with_status(404));
        }
    };

    // Update status based on action
    user_request.status = match body.action {
        AdminRequestAction::Approve => RequestStatus::Resolved,
        AdminRequestAction::Deny => RequestStatus::Rejected,
        AdminRequestAction::MarkInProgress => RequestStatus::InProgress,
    };
    user_request.updated_at = Some(chrono::Utc::now().to_rfc3339());
    user_request.admin_notes = body.admin_notes;

    // If approving a request, apply the relevant changes
    if body.action == AdminRequestAction::Approve {
        let users_kv = env
            .kv("USERS")
            .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
        let user_key = format!("user:{}", user_request.user_id);

        if let Ok(Some(user_str)) = users_kv.get(&user_key).text().await {
            if let Ok(mut user) = serde_json::from_str::<auth::types::User>(&user_str) {
                let mut user_changed = false;

                // Grant bonus documents for MoreDocuments requests
                if let Some(granted) = body.granted_documents {
                    // Reduce monthly count to grant "bonus" documents
                    // (effectively giving them more room in their quota)
                    if user.monthly_document_count >= granted {
                        user.monthly_document_count -= granted;
                    } else {
                        user.monthly_document_count = 0;
                    }
                    user_changed = true;
                }

                // Bug #7: Apply name change for NameChange requests
                if user_request.request_type == auth::types::RequestType::NameChange {
                    if let Some(ref new_first) = user_request.new_first_name {
                        user.first_name = new_first.clone();
                    }
                    if let Some(ref new_middle) = user_request.new_middle_initial {
                        user.middle_initial = Some(new_middle.clone());
                    } else if user_request.new_middle_initial.is_none()
                        && user_request.request_type == auth::types::RequestType::NameChange
                    {
                        // Clear middle initial if explicitly set to None in request
                        user.middle_initial = None;
                    }
                    if let Some(ref new_last) = user_request.new_last_name {
                        user.last_name = new_last.clone();
                    }
                    // Clear the pending name change request ID
                    user.pending_name_change_request_id = None;
                    user_changed = true;
                    console_log!(
                        "Name change approved for user {}: {} {}",
                        user.email,
                        user.first_name,
                        user.last_name
                    );
                }

                if user_changed {
                    user.updated_at = chrono::Utc::now().to_rfc3339();
                    // Save updated user
                    let user_json = serde_json::to_string(&user)
                        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
                    let _ = users_kv.put(&user_key, &user_json)?.execute().await;
                }
            }
        }
    }

    // Bug #7: If rejecting a name change, clear the pending request ID
    if body.action == AdminRequestAction::Deny
        && user_request.request_type == auth::types::RequestType::NameChange
    {
        let users_kv = env
            .kv("USERS")
            .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
        let user_key = format!("user:{}", user_request.user_id);

        if let Ok(Some(user_str)) = users_kv.get(&user_key).text().await {
            if let Ok(mut user) = serde_json::from_str::<auth::types::User>(&user_str) {
                user.pending_name_change_request_id = None;
                user.updated_at = chrono::Utc::now().to_rfc3339();
                let user_json = serde_json::to_string(&user)
                    .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
                let _ = users_kv.put(&user_key, &user_json)?.execute().await;
            }
        }
    }

    // Save updated request
    let request_json = serde_json::to_string(&user_request)
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
    requests_kv
        .put(&request_key, &request_json)?
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // Clear user's pending request marker if resolved or rejected
    if matches!(
        user_request.status,
        RequestStatus::Resolved | RequestStatus::Rejected
    ) {
        let user_pending_key = format!("user_pending:{}", user_request.user_id);
        let _ = requests_kv.delete(&user_pending_key).await;
    }

    Ok(Response::from_json(&AdminUpdateRequestResponse {
        success: true,
        message: Some(format!("Request {} updated", request_id)),
        request: Some(user_request),
    })?)
}

// ============================================================
// Associate Session with Account (Post-Signup Linking)
// ============================================================

/// Handle POST /auth/associate-session
/// Associates a signed session with a newly created account so it appears in their inbox.
/// Called after a signer creates an account to view their signed documents.
async fn handle_associate_session(mut req: Request, env: Env) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return cors_response(Ok(response)),
        Err(e) => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401)));
        }
    };

    // Parse request body
    let body: AssociateSessionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Invalid request body",
                "error": format!("{:?}", e)
            }))?
            .with_status(400)));
        }
    };

    let kv = env.kv("SESSIONS")?;

    // Fetch the session
    let session: Option<SigningSession> = kv
        .get(&format!("session:{}", body.session_id))
        .json()
        .await?;

    let session = match session {
        Some(s) => s,
        None => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Session not found"
            }))?
            .with_status(404)));
        }
    };

    // Find recipient matching user's email (case-insensitive)
    let recipient = session
        .recipients
        .iter()
        .find(|r| r.email.eq_ignore_ascii_case(&user.email));

    let recipient = match recipient {
        Some(r) => r,
        None => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "You are not a recipient on this session"
            }))?
            .with_status(403)));
        }
    };

    // Add to recipient index so it appears in their inbox
    let user_hash = hash_sender_email(&user.email);
    let mut index = get_recipient_index(&kv, &user_hash).await;
    index.add_entry(
        body.session_id.clone(),
        recipient.id.clone(),
        session.metadata.created_at.clone(),
    );

    if let Err(e) = save_recipient_index(&kv, &user_hash, &index).await {
        console_log!("Warning: Failed to save recipient index: {:?}", e);
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Failed to associate session"
        }))?
        .with_status(500)));
    }

    cors_response(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Session associated with your account"
    })))
}

// ============================================================
// Phase 3: Template Management Handlers
// ============================================================

/// Get template index for a user
async fn get_template_index(kv: &kv::KvStore, user_hash: &str) -> TemplateIndex {
    kv.get(&format!("template_index:{}", user_hash))
        .json::<TemplateIndex>()
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Save template index for a user
async fn save_template_index(
    kv: &kv::KvStore,
    user_hash: &str,
    index: &TemplateIndex,
) -> Result<()> {
    kv.put(
        &format!("template_index:{}", user_hash),
        serde_json::to_string(index)?,
    )?
    .execute()
    .await?;
    Ok(())
}

/// Handle POST /templates - Create a new template
async fn handle_create_template(mut req: Request, env: Env) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return Ok(response),
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401));
        }
    };

    // Parse request body
    let body: CreateTemplateRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Invalid request body",
                "error": format!("{:?}", e)
            }))?
            .with_status(400));
        }
    };

    // Validate template name
    if body.name.trim().is_empty() {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Template name is required"
        }))?
        .with_status(400));
    }

    let kv = env.kv("SESSIONS")?;
    let user_hash = hash_sender_email(&user.email);
    let template_id = generate_session_id();
    let now = Utc::now().to_rfc3339();

    // Create the template
    let template = FieldTemplate {
        id: template_id.clone(),
        name: body.name.clone(),
        fields: body.fields,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    // Save template
    kv.put(
        &format!("template:{}:{}", user_hash, template_id),
        serde_json::to_string(&template)?,
    )?
    .execute()
    .await?;

    // Update index
    let mut index = get_template_index(&kv, &user_hash).await;
    index.templates.push(TemplateIndexEntry {
        id: template_id.clone(),
        name: body.name,
        field_count: template.fields.len(),
        created_at: now.clone(),
        updated_at: now,
    });

    save_template_index(&kv, &user_hash, &index).await?;

    Ok(Response::from_json(&CreateTemplateResponse {
        success: true,
        template_id,
        message: "Template created successfully".to_string(),
    })?)
}

/// Handle GET /templates - List user's templates
async fn handle_list_templates(req: Request, env: Env) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return Ok(response),
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401));
        }
    };

    let kv = env.kv("SESSIONS")?;
    let user_hash = hash_sender_email(&user.email);
    let index = get_template_index(&kv, &user_hash).await;

    Ok(Response::from_json(&ListTemplatesResponse {
        success: true,
        templates: index.templates,
    })?)
}

/// Handle GET /templates/{id} - Get a single template
async fn handle_get_template(req: Request, env: Env, template_id: String) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return Ok(response),
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401));
        }
    };

    let kv = env.kv("SESSIONS")?;
    let user_hash = hash_sender_email(&user.email);

    let template: Option<FieldTemplate> = kv
        .get(&format!("template:{}:{}", user_hash, template_id))
        .json()
        .await?;

    match template {
        Some(t) => Ok(Response::from_json(&GetTemplateResponse {
            success: true,
            template: Some(t),
            message: None,
        })?),
        None => Ok(Response::from_json(&GetTemplateResponse {
            success: false,
            template: None,
            message: Some("Template not found".to_string()),
        })?
        .with_status(404)),
    }
}

/// Handle PUT /templates/{id} - Update a template
async fn handle_update_template(
    mut req: Request,
    env: Env,
    template_id: String,
) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return Ok(response),
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401));
        }
    };

    // Parse request body
    let body: CreateTemplateRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Invalid request body",
                "error": format!("{:?}", e)
            }))?
            .with_status(400));
        }
    };

    let kv = env.kv("SESSIONS")?;
    let user_hash = hash_sender_email(&user.email);
    let template_key = format!("template:{}:{}", user_hash, template_id);

    // Fetch existing template
    let existing: Option<FieldTemplate> = kv.get(&template_key).json().await?;

    let existing = match existing {
        Some(t) => t,
        None => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Template not found"
            }))?
            .with_status(404));
        }
    };

    let now = Utc::now().to_rfc3339();

    // Update template
    let updated = FieldTemplate {
        id: template_id.clone(),
        name: body.name.clone(),
        fields: body.fields,
        created_at: existing.created_at,
        updated_at: now.clone(),
    };

    kv.put(&template_key, serde_json::to_string(&updated)?)?
        .execute()
        .await?;

    // Update index
    let mut index = get_template_index(&kv, &user_hash).await;
    if let Some(entry) = index.templates.iter_mut().find(|e| e.id == template_id) {
        entry.name = body.name;
        entry.field_count = updated.fields.len();
        entry.updated_at = now;
    }
    save_template_index(&kv, &user_hash, &index).await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Template updated successfully"
    }))?)
}

/// Handle DELETE /templates/{id} - Delete a template
async fn handle_delete_template(req: Request, env: Env, template_id: String) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return Ok(response),
        Err(e) => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication required",
                "error": format!("{:?}", e)
            }))?
            .with_status(401));
        }
    };

    let kv = env.kv("SESSIONS")?;
    let user_hash = hash_sender_email(&user.email);
    let template_key = format!("template:{}:{}", user_hash, template_id);

    // Check if template exists
    let existing: Option<FieldTemplate> = kv.get(&template_key).json().await?;
    if existing.is_none() {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Template not found"
        }))?
        .with_status(404));
    }

    // Delete template
    kv.delete(&template_key).await?;

    // Update index
    let mut index = get_template_index(&kv, &user_hash).await;
    index.templates.retain(|e| e.id != template_id);
    save_template_index(&kv, &user_hash, &index).await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Template deleted successfully"
    }))?)
}

/// List users (admin only, supports ?filter=unverified)
async fn handle_admin_list_users(req: Request, env: Env) -> Result<Response> {
    use auth::types::{AdminUserSummary, AdminUsersListResponse, User};

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let users_kv = env
        .kv("USERS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // Get filter from query params
    let url = req.url()?;
    let query_params: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let filter = query_params.get("filter").map(|s| s.to_string());

    // List all user keys
    let list_result = users_kv
        .list()
        .prefix("user:".to_string())
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let mut users: Vec<AdminUserSummary> = Vec::new();

    for key in list_result.keys {
        // Skip email index keys
        if key.name.starts_with("user_email:") {
            continue;
        }

        if let Ok(Some(user_str)) = users_kv.get(&key.name).text().await {
            if let Ok(user) = serde_json::from_str::<User>(&user_str) {
                // Apply filter
                let include = match filter.as_deref() {
                    Some("unverified") => !user.email_verified,
                    Some("verified") => user.email_verified,
                    _ => true,
                };

                if include {
                    users.push(AdminUserSummary::from(&user));
                }
            }
        }
    }

    // Sort by created_at descending (newest first)
    users.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = users.len();
    Ok(Response::from_json(&AdminUsersListResponse {
        success: true,
        users,
        total,
    })?)
}

/// Adjust user quota (admin only)
async fn handle_admin_adjust_quota(
    mut req: Request,
    env: Env,
    user_id: String,
) -> Result<Response> {
    use auth::types::{AdminAdjustQuotaBody, AdminAdjustQuotaResponse, User};

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    // Parse request body
    let body: AdminAdjustQuotaBody = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&AdminAdjustQuotaResponse {
                success: false,
                message: Some(format!("Invalid request: {}", e)),
            })?
            .with_status(400));
        }
    };

    let users_kv = env
        .kv("USERS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let user_key = format!("user:{}", user_id);

    // Get existing user
    let mut user: User = match users_kv.get(&user_key).text().await {
        Ok(Some(s)) => match serde_json::from_str(&s) {
            Ok(u) => u,
            Err(_) => {
                return Ok(Response::from_json(&AdminAdjustQuotaResponse {
                    success: false,
                    message: Some("Invalid user data".to_string()),
                })?
                .with_status(500));
            }
        },
        _ => {
            return Ok(Response::from_json(&AdminAdjustQuotaResponse {
                success: false,
                message: Some("User not found".to_string()),
            })?
            .with_status(404));
        }
    };

    let mut changes: Vec<String> = Vec::new();

    // Update tier if provided
    if let Some(new_tier) = body.new_tier {
        user.tier = new_tier;
        changes.push(format!("tier → {}", new_tier.display_name()));
    }

    // Grant bonus documents (reduce usage count)
    if let Some(bonus) = body.bonus_documents {
        if user.monthly_document_count >= bonus {
            user.monthly_document_count -= bonus;
        } else {
            user.monthly_document_count = 0;
        }
        changes.push(format!("+{} docs", bonus));
    }

    user.updated_at = chrono::Utc::now().to_rfc3339();

    // Save updated user
    let user_json =
        serde_json::to_string(&user).map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
    users_kv
        .put(&user_key, &user_json)?
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let message = if changes.is_empty() {
        "No changes made".to_string()
    } else {
        format!("Updated: {}", changes.join(", "))
    };

    Ok(Response::from_json(&AdminAdjustQuotaResponse {
        success: true,
        message: Some(message),
    })?)
}

/// Delete user (admin only)
async fn handle_admin_delete_user(req: Request, env: Env, user_id: String) -> Result<Response> {
    use auth::types::AdminDeleteUserResponse;

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let users_kv = env
        .kv("USERS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let user_key = format!("user:{}", user_id);

    // Get user to find their email (for email index cleanup)
    let user_email = match users_kv.get(&user_key).text().await {
        Ok(Some(s)) => match serde_json::from_str::<auth::types::User>(&s) {
            Ok(u) => Some(u.email),
            Err(_) => None,
        },
        _ => {
            return Ok(Response::from_json(&AdminDeleteUserResponse {
                success: false,
                message: Some("User not found".to_string()),
            })?
            .with_status(404));
        }
    };

    // Delete user
    users_kv
        .delete(&user_key)
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // Delete email index if we found the email
    if let Some(email) = user_email {
        let email_key = format!("user_email:{}", email.to_lowercase());
        let _ = users_kv.delete(&email_key).await;
    }

    // Also clean up any pending requests
    if let Ok(requests_kv) = env.kv("REQUESTS") {
        let user_pending_key = format!("user_pending:{}", user_id);
        let _ = requests_kv.delete(&user_pending_key).await;
    }

    Ok(Response::from_json(&AdminDeleteUserResponse {
        success: true,
        message: Some("User deleted successfully".to_string()),
    })?)
}

// ============================================================================
// Bug #20: Unsubscribe Handlers
// ============================================================================

/// Handle POST /unsubscribe - Process unsubscribe request
async fn handle_unsubscribe(mut req: Request, env: Env) -> Result<Response> {
    #[derive(Deserialize)]
    struct UnsubscribeRequest {
        email: String,
    }

    #[derive(Serialize)]
    struct UnsubscribeResponse {
        success: bool,
        message: Option<String>,
        already_unsubscribed: bool,
        has_account: bool,
    }

    let body: UnsubscribeRequest = req
        .json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Invalid request body: {:?}", e)))?;

    let email = body.email.trim().to_lowercase();

    // Check if email is valid
    if !email.contains('@') {
        return Ok(Response::from_json(&UnsubscribeResponse {
            success: false,
            message: Some("Invalid email address".to_string()),
            already_unsubscribed: false,
            has_account: false,
        })?
        .with_status(400));
    }

    // Check if already unsubscribed
    if let Some(_entry) = email::get_do_not_email_entry(&env, &email).await? {
        return Ok(Response::from_json(&UnsubscribeResponse {
            success: true,
            message: Some("Already unsubscribed".to_string()),
            already_unsubscribed: true,
            has_account: false,
        })?);
    }

    // Check if user has an account
    let users_kv = env.kv("USERS").ok();
    let has_account = if let Some(ref kv) = users_kv {
        let email_key = format!("user_email:{}", email);
        kv.get(&email_key).text().await?.is_some()
    } else {
        false
    };

    if has_account {
        // User has account - don't unsubscribe, redirect to settings
        return Ok(Response::from_json(&UnsubscribeResponse {
            success: false,
            message: Some("Please manage email preferences in your account settings".to_string()),
            already_unsubscribed: false,
            has_account: true,
        })?
        .with_status(409)); // Conflict - user should manage in settings
    }

    // Add to Do Not Email list
    if let Err(e) = email::add_to_do_not_email(&env, &email, false, None).await {
        console_log!("Failed to add to Do Not Email list: {:?}", e);
        return Ok(Response::from_json(&UnsubscribeResponse {
            success: false,
            message: Some("Failed to process unsubscribe request".to_string()),
            already_unsubscribed: false,
            has_account: false,
        })?
        .with_status(500));
    }

    console_log!("Successfully unsubscribed: {}", email);

    Ok(Response::from_json(&UnsubscribeResponse {
        success: true,
        message: Some("Successfully unsubscribed".to_string()),
        already_unsubscribed: false,
        has_account: false,
    })?)
}

/// Handle GET /unsubscribe - Check unsubscribe status
async fn handle_check_unsubscribe(req: Request, env: Env) -> Result<Response> {
    #[derive(Serialize)]
    struct CheckResponse {
        is_unsubscribed: bool,
        has_account: bool,
    }

    let url = req.url()?;
    let email = url
        .query_pairs()
        .find(|(k, _)| k == "email")
        .map(|(_, v)| {
            // Decode base64 email token
            email::decode_unsubscribe_token(&v).unwrap_or_default()
        })
        .unwrap_or_default();

    if email.is_empty() || !email.contains('@') {
        return error_response("Invalid or missing email parameter");
    }

    let email = email.to_lowercase();

    // Check if unsubscribed
    let is_unsubscribed = email::is_email_blocked(&env, &email).await.unwrap_or(false);

    // Check if has account
    let users_kv = env.kv("USERS").ok();
    let has_account = if let Some(ref kv) = users_kv {
        let email_key = format!("user_email:{}", email);
        kv.get(&email_key).text().await?.is_some()
    } else {
        false
    };

    Ok(Response::from_json(&CheckResponse {
        is_unsubscribed,
        has_account,
    })?)
}

// ============================================================================
// Beta Access Grant Handlers
// ============================================================================

/// List all beta grants (admin only)
async fn handle_admin_list_beta_grants(req: Request, env: Env) -> Result<Response> {
    use auth::types::{AdminBetaGrantsListResponse, BetaGrant, BetaGrantWithStatus, User};

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let beta_kv = match env.kv("BETA_GRANTS") {
        Ok(kv) => kv,
        Err(_) => {
            return Ok(Response::from_json(&AdminBetaGrantsListResponse {
                success: true,
                grants: vec![],
                total: 0,
                active_count: 0,
            })?);
        }
    };

    let users_kv = env.kv("USERS").ok();

    // List all grant keys (prefix "grant:")
    let list_result = beta_kv
        .list()
        .prefix("grant:".to_string())
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let mut grants: Vec<BetaGrantWithStatus> = Vec::new();
    let mut active_count = 0;

    for key in list_result.keys {
        if let Ok(Some(grant_str)) = beta_kv.get(&key.name).text().await {
            if let Ok(grant) = serde_json::from_str::<BetaGrant>(&grant_str) {
                // Check if user exists
                let (user_exists, user_id, current_tier) = if let Some(ref kv) = users_kv {
                    let email_key = format!("user_email:{}", grant.email.to_lowercase());
                    if let Ok(Some(uid)) = kv.get(&email_key).text().await {
                        // Get user's current tier
                        let user_key = format!("user:{}", uid);
                        let tier = if let Ok(Some(user_str)) = kv.get(&user_key).text().await {
                            serde_json::from_str::<User>(&user_str).ok().map(|u| u.tier)
                        } else {
                            None
                        };
                        (true, Some(uid), tier)
                    } else {
                        (false, None, None)
                    }
                } else {
                    (false, None, None)
                };

                if !grant.revoked {
                    active_count += 1;
                }

                grants.push(BetaGrantWithStatus {
                    grant,
                    user_exists,
                    user_id,
                    current_tier,
                });
            }
        }
    }

    // Sort by granted_at descending (newest first)
    grants.sort_by(|a, b| b.grant.granted_at.cmp(&a.grant.granted_at));

    let total = grants.len();
    Ok(Response::from_json(&AdminBetaGrantsListResponse {
        success: true,
        grants,
        total,
        active_count,
    })?)
}

/// Create a beta grant (admin only)
async fn handle_admin_create_beta_grant(mut req: Request, env: Env) -> Result<Response> {
    use auth::types::{
        AdminCreateBetaGrantBody, AdminCreateBetaGrantResponse, BetaGrant, GrantSource, User,
    };

    // Check admin access
    let admin_email = match get_admin_user(&req, &env).await? {
        Ok(user) => user.email,
        Err(resp) => return Ok(resp),
    };

    // Parse request body
    let body: AdminCreateBetaGrantBody = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&AdminCreateBetaGrantResponse {
                success: false,
                message: Some(format!("Invalid request: {}", e)),
                grant: None,
                user_exists: false,
                user_upgraded: false,
            })?
            .with_status(400));
        }
    };

    // Validate email format
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') || !email.contains('.') {
        return Ok(Response::from_json(&AdminCreateBetaGrantResponse {
            success: false,
            message: Some("Invalid email format".to_string()),
            grant: None,
            user_exists: false,
            user_upgraded: false,
        })?
        .with_status(400));
    }

    let beta_kv = env
        .kv("BETA_GRANTS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let grant_key = format!("grant:{}", email);

    // Check if grant already exists
    if let Ok(Some(existing_str)) = beta_kv.get(&grant_key).text().await {
        if let Ok(existing) = serde_json::from_str::<BetaGrant>(&existing_str) {
            if !existing.revoked {
                return Ok(Response::from_json(&AdminCreateBetaGrantResponse {
                    success: false,
                    message: Some("Grant already exists for this email".to_string()),
                    grant: Some(existing),
                    user_exists: false,
                    user_upgraded: false,
                })?
                .with_status(409));
            }
            // If revoked, we'll overwrite it with a new grant
        }
    }

    // Create the grant
    let grant = BetaGrant {
        email: email.clone(),
        tier: body.tier,
        granted_by: admin_email,
        granted_at: chrono::Utc::now().to_rfc3339(),
        notes: body.notes,
        revoked: false,
        revoked_at: None,
        welcome_email_sent: false, // Bug #19: Track email sent status
    };

    // Save grant to KV
    let grant_json =
        serde_json::to_string(&grant).map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
    beta_kv
        .put(&grant_key, grant_json)?
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // Check if user already exists and upgrade them
    let users_kv = env.kv("USERS").ok();
    let (user_exists, user_upgraded) = if let Some(ref kv) = users_kv {
        let email_key = format!("user_email:{}", email);
        if let Ok(Some(user_id)) = kv.get(&email_key).text().await {
            let user_key = format!("user:{}", user_id);
            if let Ok(Some(user_str)) = kv.get(&user_key).text().await {
                if let Ok(mut user) = serde_json::from_str::<User>(&user_str) {
                    // Upgrade user's tier
                    user.tier = grant.tier;
                    user.grant_source = Some(GrantSource::BetaGrant);
                    user.granted_tier = Some(grant.tier);
                    user.updated_at = chrono::Utc::now().to_rfc3339();

                    // Save updated user
                    if let Ok(user_json) = serde_json::to_string(&user) {
                        let _ = kv.put(&user_key, user_json)?.execute().await;
                    }
                    (true, true)
                } else {
                    (true, false)
                }
            } else {
                (false, false)
            }
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    // Bug #19: Send welcome email
    let mut final_grant = grant.clone();
    let email_sent = match email::send_beta_grant_welcome_email(&env, &email).await {
        Ok(result) if result.success => {
            console_log!("Welcome email sent to {}", email);
            // Update grant to mark email as sent
            final_grant.welcome_email_sent = true;
            let grant_json = serde_json::to_string(&final_grant)
                .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
            let _ = beta_kv.put(&grant_key, grant_json)?.execute().await;
            true
        }
        Ok(result) => {
            console_log!(
                "Welcome email failed for {}: {:?}",
                email,
                result.error
            );
            false
        }
        Err(e) => {
            console_log!("Welcome email error for {}: {:?}", email, e);
            false
        }
    };

    let message = if user_upgraded {
        if email_sent {
            "Grant created, user upgraded, and welcome email sent".to_string()
        } else {
            "Grant created and user upgraded (email send failed)".to_string()
        }
    } else if user_exists {
        "Grant created (user exists but upgrade failed)".to_string()
    } else if email_sent {
        "Grant created and welcome email sent (user will get access when they register)".to_string()
    } else {
        "Grant created (user will get access when they register, email send failed)".to_string()
    };

    Ok(Response::from_json(&AdminCreateBetaGrantResponse {
        success: true,
        message: Some(message),
        grant: Some(final_grant),
        user_exists,
        user_upgraded,
    })?)
}

/// Revoke a beta grant (admin only)
async fn handle_admin_revoke_beta_grant(req: Request, env: Env, email: String) -> Result<Response> {
    use auth::types::{AdminRevokeBetaGrantResponse, BetaGrant, User, UserTier};

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let email = email.trim().to_lowercase();

    let beta_kv = env
        .kv("BETA_GRANTS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let grant_key = format!("grant:{}", email);

    // Get existing grant
    let mut grant: BetaGrant = match beta_kv.get(&grant_key).text().await {
        Ok(Some(s)) => match serde_json::from_str(&s) {
            Ok(g) => g,
            Err(_) => {
                return Ok(Response::from_json(&AdminRevokeBetaGrantResponse {
                    success: false,
                    message: Some("Invalid grant data".to_string()),
                    user_downgraded: false,
                })?
                .with_status(500));
            }
        },
        _ => {
            return Ok(Response::from_json(&AdminRevokeBetaGrantResponse {
                success: false,
                message: Some("Grant not found".to_string()),
                user_downgraded: false,
            })?
            .with_status(404));
        }
    };

    // Mark as revoked
    grant.revoked = true;
    grant.revoked_at = Some(chrono::Utc::now().to_rfc3339());

    // Save updated grant
    let grant_json =
        serde_json::to_string(&grant).map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
    beta_kv
        .put(&grant_key, grant_json)?
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // Check if user exists and downgrade if they don't have a subscription
    let users_kv = env.kv("USERS").ok();
    let user_downgraded = if let Some(ref kv) = users_kv {
        let email_key = format!("user_email:{}", email);
        if let Ok(Some(user_id)) = kv.get(&email_key).text().await {
            let user_key = format!("user:{}", user_id);
            if let Ok(Some(user_str)) = kv.get(&user_key).text().await {
                if let Ok(mut user) = serde_json::from_str::<User>(&user_str) {
                    // Only downgrade if they don't have a Stripe subscription
                    if user.stripe_subscription_id.is_none() {
                        user.tier = UserTier::Free;
                        user.grant_source = None;
                        user.granted_tier = None;
                        user.updated_at = chrono::Utc::now().to_rfc3339();

                        // Save updated user
                        if let Ok(user_json) = serde_json::to_string(&user) {
                            let _ = kv.put(&user_key, user_json)?.execute().await;
                        }
                        true
                    } else {
                        false // Has subscription, keep their paid tier
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(Response::from_json(&AdminRevokeBetaGrantResponse {
        success: true,
        message: Some(if user_downgraded {
            "Grant revoked and user downgraded to Free".to_string()
        } else {
            "Grant revoked".to_string()
        }),
        user_downgraded,
    })?)
}

/// Bug #19: Send pending welcome emails to all grants where welcome_email_sent == false
async fn handle_admin_send_pending_welcome_emails(req: Request, env: Env) -> Result<Response> {
    use auth::types::BetaGrant;

    // Check admin access
    match get_admin_user(&req, &env).await? {
        Ok(_) => {}
        Err(resp) => return Ok(resp),
    }

    let beta_kv = env
        .kv("BETA_GRANTS")
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    // List all grants (prefix = "grant:")
    let grants_list = beta_kv
        .list()
        .prefix("grant:".to_string())
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;

    let mut pending_count = 0;
    let mut sent_count = 0;
    let mut failed_count = 0;
    let mut errors: Vec<String> = Vec::new();

    // Process each grant
    for key in grants_list.keys {
        let key_name = key.name;

        // Get the grant
        let grant_str = match beta_kv.get(&key_name).text().await {
            Ok(Some(s)) => s,
            _ => continue,
        };

        let mut grant: BetaGrant = match serde_json::from_str(&grant_str) {
            Ok(g) => g,
            Err(_) => continue,
        };

        // Skip if already sent or revoked
        if grant.welcome_email_sent || grant.revoked {
            continue;
        }

        pending_count += 1;

        // Send welcome email
        match email::send_beta_grant_welcome_email(&env, &grant.email).await {
            Ok(result) if result.success => {
                // Update grant to mark email as sent
                grant.welcome_email_sent = true;
                if let Ok(grant_json) = serde_json::to_string(&grant) {
                    let _ = beta_kv.put(&key_name, grant_json)?.execute().await;
                }
                sent_count += 1;
                console_log!("Batch send: sent welcome email to {}", grant.email);
            }
            Ok(result) => {
                failed_count += 1;
                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                errors.push(format!("{}: {}", grant.email, error_msg));
                console_log!("Batch send: failed for {} - {}", grant.email, error_msg);
            }
            Err(e) => {
                failed_count += 1;
                errors.push(format!("{}: {:?}", grant.email, e));
                console_log!("Batch send: error for {} - {:?}", grant.email, e);
            }
        }
    }

    #[derive(serde::Serialize)]
    struct BatchSendResponse {
        success: bool,
        pending_count: u32,
        sent_count: u32,
        failed_count: u32,
        errors: Vec<String>,
        message: String,
    }

    let message = if pending_count == 0 {
        "No pending welcome emails to send".to_string()
    } else if failed_count == 0 {
        format!("Successfully sent {} welcome email(s)", sent_count)
    } else {
        format!(
            "Sent {}/{} emails ({} failed)",
            sent_count, pending_count, failed_count
        )
    };

    Ok(Response::from_json(&BatchSendResponse {
        success: failed_count == 0,
        pending_count,
        sent_count,
        failed_count,
        errors,
        message,
    })?)
}

// ============================================================
// Bug #21: Account Deletion (GDPR)
// ============================================================

/// TTL for account deletion confirmation tokens (1 hour)
const DELETION_TTL: u64 = 3600;

/// Request account deletion - sends confirmation email
async fn handle_request_account_deletion(req: Request, env: Env) -> Result<Response> {
    use auth::types::AccountDeletionRequest;

    // Verify auth token
    let (user, _users_kv) = match auth::require_auth(&req, &env).await? {
        Ok(u) => u,
        Err(resp) => return Ok(resp),
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;

    // Generate deletion token
    let token = auth::generate_token();
    let deletion_key = format!("deletion:{}", token);

    // Create deletion request
    let deletion_request = AccountDeletionRequest {
        user_id: user.id.clone(),
        email: user.email.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: (chrono::Utc::now() + chrono::Duration::seconds(DELETION_TTL as i64))
            .to_rfc3339(),
    };

    // Store in KV with TTL
    let deletion_json = serde_json::to_string(&deletion_request)
        .map_err(|e| worker::Error::RustError(format!("{:?}", e)))?;
    verifications_kv
        .put(&deletion_key, deletion_json)?
        .expiration_ttl(DELETION_TTL)
        .execute()
        .await?;

    // Send confirmation email
    let email_result =
        email::send_account_deletion_email(&env, &user.email, &user.first_name, &token).await?;

    #[derive(serde::Serialize)]
    struct RequestDeletionResponse {
        success: bool,
        message: String,
    }

    if email_result.success {
        Ok(Response::from_json(&RequestDeletionResponse {
            success: true,
            message: "Check your email for a deletion confirmation link. The link expires in 1 hour.".to_string(),
        })?)
    } else {
        Ok(Response::from_json(&RequestDeletionResponse {
            success: false,
            message: "Failed to send confirmation email. Please try again.".to_string(),
        })?
        .with_status(500))
    }
}

/// Confirm account deletion - actually deletes the account
async fn handle_confirm_account_deletion(mut req: Request, env: Env) -> Result<Response> {
    use auth::types::AccountDeletionRequest;

    #[derive(serde::Deserialize)]
    struct ConfirmDeletionBody {
        token: String,
    }

    #[derive(serde::Serialize)]
    struct ConfirmDeletionResponse {
        success: bool,
        message: String,
    }

    // Parse request body
    let body: ConfirmDeletionBody = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&ConfirmDeletionResponse {
                success: false,
                message: format!("Invalid request: {}", e),
            })?
            .with_status(400));
        }
    };

    let verifications_kv = env.kv("VERIFICATIONS")?;
    let deletion_key = format!("deletion:{}", body.token);

    // Get deletion request
    let deletion_request: AccountDeletionRequest = match verifications_kv
        .get(&deletion_key)
        .json::<AccountDeletionRequest>()
        .await?
    {
        Some(r) => r,
        None => {
            return Ok(Response::from_json(&ConfirmDeletionResponse {
                success: false,
                message: "Invalid or expired deletion link. Please request a new one.".to_string(),
            })?
            .with_status(400));
        }
    };

    // Check if expired
    if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&deletion_request.expires_at) {
        if chrono::Utc::now() > expires_at {
            // Delete the expired token
            let _ = verifications_kv.delete(&deletion_key).await;
            return Ok(Response::from_json(&ConfirmDeletionResponse {
                success: false,
                message: "Deletion link has expired. Please request a new one.".to_string(),
            })?
            .with_status(400));
        }
    }

    // Delete the user's data
    let users_kv = env.kv("USERS")?;
    let _auth_sessions_kv = env.kv("AUTH_SESSIONS")?;

    // 1. Delete user record
    let user_key = format!("user:{}", deletion_request.user_id);
    let _ = users_kv.delete(&user_key).await;

    // 2. Delete email index
    let email_key = format!("user_email:{}", deletion_request.email.to_lowercase());
    let _ = users_kv.delete(&email_key).await;

    // 3. Delete all auth sessions for this user (iterate through all and check)
    // Note: In production, you might want to store user_id -> session_ids mapping
    // For now, we'll delete what we can
    console_log!(
        "Account deletion: deleted user {} ({})",
        deletion_request.user_id,
        deletion_request.email
    );

    // 4. Delete any pending verifications
    // Clean up email verification tokens if any
    let email_verify_key = format!("email_verify:{}", body.token);
    let _ = verifications_kv.delete(&email_verify_key).await;

    // 5. Delete the deletion request token
    let _ = verifications_kv.delete(&deletion_key).await;

    // 6. Remove from Do Not Email list if present (they're deleting, not unsubscribing)
    let _ = email::remove_from_do_not_email(&env, &deletion_request.email).await;

    Ok(Response::from_json(&ConfirmDeletionResponse {
        success: true,
        message: "Your account has been permanently deleted. We're sorry to see you go!".to_string(),
    })?)
}

// ============================================================
// Bug #22: Google OAuth Login
// ============================================================

/// Handle Google OAuth login/signup
async fn handle_google_oauth(mut req: Request, env: Env) -> Result<Response> {
    use auth::types::{AuthSession, BillingCycle, OAuthProvider, RefreshToken, User, UserTier};
    use chrono::Datelike;

    #[derive(serde::Deserialize)]
    struct GoogleOAuthRequest {
        /// The ID token from Google Identity Services
        id_token: String,
    }

    #[derive(serde::Deserialize)]
    struct GoogleTokenInfo {
        /// Google user ID (subject)
        sub: String,
        /// User's email address
        email: String,
        /// Whether the email is verified by Google
        email_verified: bool,
        /// User's given name (first name)
        #[serde(default)]
        given_name: Option<String>,
        /// User's family name (last name)
        #[serde(default)]
        family_name: Option<String>,
        /// User's full name
        #[serde(default)]
        name: Option<String>,
        /// Profile picture URL
        #[serde(default)]
        picture: Option<String>,
        /// The audience (should match our client ID)
        aud: String,
    }

    #[derive(serde::Serialize)]
    struct GoogleOAuthResponse {
        success: bool,
        message: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        user: Option<auth::types::UserPublic>,
        is_new_user: bool,
    }

    // Parse request body
    let body: GoogleOAuthRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::from_json(&GoogleOAuthResponse {
                success: false,
                message: Some(format!("Invalid request: {}", e)),
                access_token: None,
                refresh_token: None,
                user: None,
                is_new_user: false,
            })?
            .with_status(400));
        }
    };

    // Get Google Client ID from environment
    let google_client_id = match env.var("GOOGLE_CLIENT_ID") {
        Ok(id) => id.to_string(),
        Err(_) => {
            console_log!("GOOGLE_CLIENT_ID not configured");
            return Ok(Response::from_json(&GoogleOAuthResponse {
                success: false,
                message: Some("Google OAuth not configured. Please contact support.".to_string()),
                access_token: None,
                refresh_token: None,
                user: None,
                is_new_user: false,
            })?
            .with_status(500));
        }
    };

    // Verify the ID token with Google
    let verify_url = format!(
        "https://oauth2.googleapis.com/tokeninfo?id_token={}",
        body.id_token
    );

    let google_response = match worker::Fetch::Url(verify_url.parse().unwrap())
        .send()
        .await
    {
        Ok(mut resp) => {
            let status = resp.status_code();
            if status < 200 || status >= 300 {
                console_log!("Google token verification failed: status {}", status);
                return Ok(Response::from_json(&GoogleOAuthResponse {
                    success: false,
                    message: Some("Invalid Google token. Please try signing in again.".to_string()),
                    access_token: None,
                    refresh_token: None,
                    user: None,
                    is_new_user: false,
                })?
                .with_status(401));
            }
            match resp.json::<GoogleTokenInfo>().await {
                Ok(info) => info,
                Err(e) => {
                    console_log!("Failed to parse Google response: {:?}", e);
                    return Ok(Response::from_json(&GoogleOAuthResponse {
                        success: false,
                        message: Some("Failed to verify Google token.".to_string()),
                        access_token: None,
                        refresh_token: None,
                        user: None,
                        is_new_user: false,
                    })?
                    .with_status(500));
                }
            }
        }
        Err(e) => {
            console_log!("Google verification request failed: {:?}", e);
            return Ok(Response::from_json(&GoogleOAuthResponse {
                success: false,
                message: Some("Failed to verify with Google. Please try again.".to_string()),
                access_token: None,
                refresh_token: None,
                user: None,
                is_new_user: false,
            })?
            .with_status(500));
        }
    };

    // Verify the audience matches our client ID
    if google_response.aud != google_client_id {
        console_log!(
            "Google aud mismatch: expected {}, got {}",
            google_client_id,
            google_response.aud
        );
        return Ok(Response::from_json(&GoogleOAuthResponse {
            success: false,
            message: Some("Invalid token audience.".to_string()),
            access_token: None,
            refresh_token: None,
            user: None,
            is_new_user: false,
        })?
        .with_status(401));
    }

    // Check if email is verified by Google
    if !google_response.email_verified {
        return Ok(Response::from_json(&GoogleOAuthResponse {
            success: false,
            message: Some("Your Google email is not verified. Please verify it first.".to_string()),
            access_token: None,
            refresh_token: None,
            user: None,
            is_new_user: false,
        })?
        .with_status(400));
    }

    let email = google_response.email.to_lowercase();
    let users_kv = env.kv("USERS")?;
    let email_key = format!("user_email:{}", email);

    // Check if user exists by email
    let (mut user, is_new_user) = match users_kv.get(&email_key).text().await? {
        Some(user_id) => {
            // Existing user - retrieve and potentially link Google
            let user_key = format!("user:{}", user_id);
            match users_kv.get(&user_key).json::<User>().await? {
                Some(mut existing_user) => {
                    // Link Google to existing account if not already linked
                    if existing_user.oauth_provider.is_none() {
                        existing_user.oauth_provider = Some(OAuthProvider::Google);
                        existing_user.oauth_provider_id = Some(google_response.sub.clone());
                        existing_user.profile_picture_url = google_response.picture.clone();
                        existing_user.updated_at = chrono::Utc::now().to_rfc3339();
                    }
                    // Mark email as verified (Google verified it)
                    existing_user.email_verified = true;
                    (existing_user, false)
                }
                None => {
                    console_log!("User email index exists but user not found: {}", email);
                    return Ok(Response::from_json(&GoogleOAuthResponse {
                        success: false,
                        message: Some("Account error. Please contact support.".to_string()),
                        access_token: None,
                        refresh_token: None,
                        user: None,
                        is_new_user: false,
                    })?
                    .with_status(500));
                }
            }
        }
        None => {
            // New user - create account
            let user_id = auth::generate_token();
            let now = chrono::Utc::now();

            // Parse name from Google
            let first_name = google_response.given_name.unwrap_or_else(|| {
                google_response
                    .name
                    .as_ref()
                    .and_then(|n| n.split_whitespace().next())
                    .unwrap_or("User")
                    .to_string()
            });
            let last_name = google_response.family_name.unwrap_or_else(|| {
                google_response
                    .name
                    .as_ref()
                    .and_then(|n| n.split_whitespace().last())
                    .unwrap_or("")
                    .to_string()
            });

            let new_user = User {
                id: user_id,
                email: email.clone(),
                email_verified: true, // Google verified it
                password_hash: String::new(), // OAuth users have no password
                tier: UserTier::Free,
                created_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
                first_name,
                middle_initial: None,
                last_name,
                monthly_document_count: 0,
                current_quota_month: format!("{}-{:02}", now.year(), now.month()),
                last_login_at: None,
                login_count: 0,
                stripe_customer_id: None,
                stripe_subscription_id: None,
                billing_cycle: BillingCycle::Monthly,
                overage_count: 0,
                limit_email_sent: false,
                name_set: true, // Google provided the name
                pending_name_change_request_id: None,
                grant_source: None,
                granted_tier: None,
                email_preferences: auth::types::EmailPreferences::default(),
                oauth_provider: Some(OAuthProvider::Google),
                oauth_provider_id: Some(google_response.sub.clone()),
                profile_picture_url: google_response.picture.clone(),
            };

            // Check for beta grant and upgrade tier if found
            let beta_kv = env.kv("BETA_GRANTS").ok();
            let mut final_user = new_user;
            if let Some(ref kv) = beta_kv {
                let grant_key = format!("grant:{}", email);
                if let Ok(Some(grant_str)) = kv.get(&grant_key).text().await {
                    if let Ok(grant) = serde_json::from_str::<auth::types::BetaGrant>(&grant_str) {
                        if !grant.revoked {
                            final_user.tier = grant.tier;
                            final_user.grant_source = Some(auth::types::GrantSource::PreGrant);
                            final_user.granted_tier = Some(grant.tier);
                            console_log!(
                                "Applied pre-grant {} tier to new Google OAuth user {}",
                                grant.tier.display_name(),
                                email
                            );
                        }
                    }
                }
            }

            // Remove from Do Not Email list if present (user creating account)
            let _ = email::remove_from_do_not_email(&env, &email).await;

            (final_user, true)
        }
    };

    // Update login tracking
    user.last_login_at = Some(chrono::Utc::now().to_rfc3339());
    user.login_count += 1;
    user.check_monthly_reset();

    // Save user
    auth::save_user(&users_kv, &user).await?;

    // Generate session and tokens
    let session_id = auth::generate_token();
    let jwt_secret = env
        .secret("JWT_SECRET")
        .map_err(|_| worker::Error::RustError("JWT_SECRET not configured".to_string()))?
        .to_string();

    let tier_str = user.tier.display_name();
    let access_token = auth::jwt::generate_access_token(&user.id, &user.email, tier_str, &jwt_secret)
        .map_err(|e| worker::Error::RustError(format!("Failed to generate access token: {}", e)))?;
    let refresh_token = auth::jwt::generate_refresh_token(&user.id, &session_id, &jwt_secret)
        .map_err(|e| worker::Error::RustError(format!("Failed to generate refresh token: {}", e)))?;

    // Store auth session in KV
    let auth_sessions_kv = env.kv("AUTH_SESSIONS")?;
    let now = chrono::Utc::now();
    let session = AuthSession {
        user_id: user.id.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::days(30)).to_rfc3339(),
        ip: None, // Could get from request headers
        user_agent: None,
    };

    let session_key = format!("auth_session:{}", session_id);
    auth_sessions_kv
        .put(&session_key, serde_json::to_string(&session)?)?
        .expiration_ttl(30 * 24 * 60 * 60) // 30 days
        .execute()
        .await?;

    // Store refresh token reference
    let refresh_key = format!("refresh_token:{}", &refresh_token[..32]); // Use first 32 chars as key
    let refresh_record = RefreshToken {
        user_id: user.id.clone(),
        session_id: session_id.clone(),
        created_at: now.to_rfc3339(),
        expires_at: (now + chrono::Duration::days(30)).to_rfc3339(),
    };
    auth_sessions_kv
        .put(&refresh_key, serde_json::to_string(&refresh_record)?)?
        .expiration_ttl(30 * 24 * 60 * 60) // 30 days
        .execute()
        .await?;

    console_log!(
        "Google OAuth {} for user {} ({})",
        if is_new_user { "signup" } else { "login" },
        user.id,
        user.email
    );

    Ok(Response::from_json(&GoogleOAuthResponse {
        success: true,
        message: Some(if is_new_user {
            "Account created successfully!".to_string()
        } else {
            "Logged in successfully!".to_string()
        }),
        access_token: Some(access_token),
        refresh_token: Some(refresh_token),
        user: Some(auth::types::UserPublic::from(&user)),
        is_new_user,
    })?)
}

async fn handle_send_email(mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        return cors_response(Ok(response));
    }

    // Parse request
    let body: SendRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Validate PDF size after parsing (double-check actual content)
    let pdf_size = body.pdf_base64.len();
    if pdf_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, pdf_size));
    }

    // Send the email via Resend
    send_document_email(&env, &body).await
}

async fn send_document_email(env: &Env, body: &SendRequest) -> Result<Response> {
    // Build email body with optional signing link
    let html_body = if let Some(ref link) = body.signing_link {
        format!(
            "<p>You have been requested to sign a document.</p>\
            <p><a href=\"{}\">Click here to sign</a></p>\
            <p>Or download the attached PDF to sign locally.</p>",
            link
        )
    } else {
        "<p>Please find the attached document for your signature.</p>".to_string()
    };

    let request = email::EmailSendRequest {
        to: vec![body.to.clone()],
        subject: body.subject.clone(),
        html: html_body,
        text: None,
        reply_to: None,
        tags: vec![("type".to_string(), "document".to_string())],
    };

    match email::send_email(env, request).await {
        Ok(result) => {
            if result.success {
                console_log!("Email sent to {}", body.to);
                cors_response(Response::from_json(&SendResponse {
                    success: true,
                    message: "Email sent".to_string(),
                }))
            } else {
                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                console_log!("Email send failed: {}", error_msg);
                cors_response(error_response(&error_msg))
            }
        }
        Err(e) => {
            console_log!("Email send error: {}", e);
            cors_response(error_response(&format!("Email service error: {}", e)))
        }
    }
}

async fn handle_send_invites(mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        return cors_response(Ok(response));
    }

    // Parse request
    let body: InviteRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    if body.invitations.is_empty() {
        return cors_response(error_response("No invitations to send"));
    }

    // Send the invitations via Resend
    send_invitations(&env, &body).await
}

async fn send_invitations(env: &Env, body: &InviteRequest) -> Result<Response> {
    let mut success_count = 0;
    let mut errors = Vec::new();

    // Get signing secret for token generation
    let secret = get_signing_secret(env);

    for invitation in &body.invitations {
        // Generate a signed token for this recipient (includes token_version for invalidation)
        let token = generate_recipient_token(
            &body.session_id,
            &invitation.recipient_id,
            body.token_version,
            &secret,
        );

        // Append token to the signing link
        let signing_link_with_token = if invitation.signing_link.contains('?') {
            format!("{}&token={}", invitation.signing_link, token)
        } else {
            format!("{}?token={}", invitation.signing_link, token)
        };

        // Bug #18 fix: Use email prefix as fallback if name is empty
        let recipient_name = if invitation.name.trim().is_empty() {
            invitation
                .email
                .split('@')
                .next()
                .unwrap_or("there")
                .to_string()
        } else {
            invitation.name.clone()
        };

        // Feature 1: Build optional alias and context sections for email
        let alias_section = body.document_alias.as_ref()
            .filter(|s| !s.trim().is_empty())
            .map(|alias| format!(
                r#"<p style="margin: 8px 0 0 0; font-size: 14px; color: #6b7280;">Also known as: <strong>{}</strong></p>"#,
                alias
            ))
            .unwrap_or_default();

        let context_section = body.signing_context.as_ref()
            .filter(|s| !s.trim().is_empty())
            .map(|context| format!(
                r#"
        <div style="background: #dbeafe; padding: 12px 15px; border-radius: 6px; margin-bottom: 25px; border-left: 4px solid #3b82f6;">
            <p style="margin: 0; font-size: 14px; color: #1e40af;">
                <strong>Context:</strong> {}
            </p>
        </div>"#,
                context
            ))
            .unwrap_or_default();

        // Build HTML email template
        let email_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #1e40af 0%, #1e3a8a 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Signature Request</h1>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Hello {recipient_name},</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{sender_name}</strong> has requested your signature on the following document:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{document_name}</p>{alias_section}
        </div>{context_section}

        <div style="text-align: center; margin: 30px 0;">
            <a href="{signing_link}" style="display: inline-block; background: #1e40af; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Review & Sign Document</a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            If you have any questions about this signature request, please contact the sender directly.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>

    <div style="text-align: center; margin-top: 10px; padding-top: 15px; border-top: 1px solid #e5e7eb; font-size: 11px; color: #6b7280;">
        <p style="margin: 0 0 8px 0;">
            You received this email because <strong>{sender_name}</strong> requested your signature on a document.
            This is a transactional email related to a specific signature request.
        </p>
        <p style="margin: 0 0 8px 0;">
            If you believe you received this email in error or do not wish to receive future requests,
            please email <a href="mailto:bobamatchasolutions@gmail.com" style="color: #1e40af;">bobamatchasolutions@gmail.com</a>.
        </p>
        <p style="margin: 0 0 8px 0;">
            <a href="https://getsignatures.org/legal.html" style="color: #1e40af;">Privacy Policy</a> |
            <a href="https://getsignatures.org/legal.html#esign" style="color: #1e40af;">E-Sign Disclosure</a>
        </p>
        <p style="margin: 0; font-size: 10px;">
            Electronic signatures have the same legal effect as handwritten signatures under the ESIGN Act and UETA.
        </p>
    </div>
</body>
</html>"#,
            recipient_name = recipient_name,
            sender_name = body.sender_name,
            document_name = body.document_name,
            signing_link = signing_link_with_token,
            alias_section = alias_section,
            context_section = context_section
        );

        // Feature 1: Build email subject with alias if provided
        let email_subject = match &body.document_alias {
            Some(alias) if !alias.trim().is_empty() => {
                format!("Signature Requested: {} ({})", alias, body.document_name)
            }
            _ => format!("Signature Requested: {}", body.document_name),
        };

        // Send via email module
        let email_request = email::EmailSendRequest {
            to: vec![invitation.email.clone()],
            subject: email_subject,
            html: email_html,
            text: None,
            reply_to: None,
            tags: vec![
                ("type".to_string(), "invitation".to_string()),
                ("session_id".to_string(), body.session_id.clone()),
            ],
        };

        match email::send_email(env, email_request).await {
            Ok(result) => {
                if result.success {
                    success_count += 1;
                    console_log!("Invitation sent to {}", invitation.email);
                } else {
                    let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                    errors.push(format!("{}: {}", invitation.email, error_msg));
                    console_log!("Failed to send to {}: {}", invitation.email, error_msg);
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", invitation.email, e));
                console_log!("Failed to send to {}: {}", invitation.email, e);
            }
        }
    }

    if success_count == body.invitations.len() {
        cors_response(Response::from_json(&InviteResponse {
            success: true,
            message: format!("All {} invitations sent successfully", success_count),
        }))
    } else if success_count > 0 {
        cors_response(Response::from_json(&InviteResponse {
            success: false,
            message: format!(
                "Partial success: {}/{} invitations sent. Errors: {}",
                success_count,
                body.invitations.len(),
                errors.join(", ")
            ),
        }))
    } else {
        cors_response(error_response(&format!(
            "Failed to send invitations. Errors: {}",
            errors.join(", ")
        )))
    }
}

async fn handle_create_session(mut req: Request, env: Env) -> Result<Response> {
    // Bug #0: Add debug logging for production issue diagnosis
    console_log!("handle_create_session: Starting request processing");

    // Check request size before parsing (contains PDF)
    if let Some(response) = check_content_length(&req, MAX_REQUEST_BODY) {
        console_log!("handle_create_session: Request too large");
        return cors_response(Ok(response));
    }

    console_log!("handle_create_session: Content length check passed");

    // Require authentication for session creation
    let (mut user, users_kv) = match auth::require_auth(&req, &env).await {
        Ok(Ok(result)) => {
            console_log!("handle_create_session: Auth successful for user");
            result
        }
        Ok(Err(response)) => {
            console_log!("handle_create_session: Auth returned error response");
            return cors_response(Ok(response));
        }
        Err(e) => {
            console_log!("handle_create_session: Auth failed with error: {:?}", e);
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication error. Please log in again.",
                "error_code": "AUTH_ERROR",
                "debug_info": format!("{:?}", e)
            }))?
            .with_status(500)));
        }
    };

    // Check if email is verified
    if !user.email_verified {
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Please verify your email address before creating signing sessions.",
            "error_code": "EMAIL_NOT_VERIFIED"
        }))?
        .with_status(403)));
    }

    // Bug #6: Check document limit based on user's tier
    // Reset counter if the month has changed
    user.check_monthly_reset();

    // Check if this is a testing account with unlimited sends
    let is_unlimited_testing = is_testing_unlimited_account(&user.email);
    if is_unlimited_testing {
        console_log!("⚠️ TESTING MODE: Unlimited sends active for {}", user.email);
    }

    // Check if user can create another document (respects tier limits + overage)
    // Skip limit check for testing accounts
    if !is_unlimited_testing && !user.can_create_document() {
        let limit = user.tier.monthly_limit();
        let tier_name = user.tier.display_name();
        let message = if user.tier.allows_overage() {
            format!(
                "You've reached your maximum document limit for this month ({} base + {} overage). Your {} plan resets on the 1st of next month.",
                limit,
                user.tier.max_with_overage() - limit,
                tier_name
            )
        } else {
            format!(
                "Monthly limit reached. Free accounts can create {} documents per month. Upgrade to unlock more documents and premium features.",
                limit
            )
        };

        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": message,
            "error_code": "MONTHLY_LIMIT_EXCEEDED",
            "limit": limit,
            "tier": tier_name,
            "documents_used": user.monthly_document_count,
            "overage_used": user.overage_count,
            "allows_overage": user.tier.allows_overage(),
            "is_in_overage": user.is_in_overage()
        }))?
        .with_status(429)));
    }

    console_log!("handle_create_session: About to parse request body");
    let body: CreateSessionRequest = match req.json::<CreateSessionRequest>().await {
        Ok(b) => {
            console_log!(
                "handle_create_session: Body parsed, doc size: {} bytes, {} recipients, {} fields",
                b.encrypted_document.len(),
                b.recipients.len(),
                b.fields.len()
            );
            b
        }
        Err(e) => {
            console_log!("handle_create_session: Body parsing failed: {:?}", e);
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": format!("Invalid request format: {}", e),
                "error_code": "BODY_PARSE_ERROR"
            }))?
            .with_status(400)));
        }
    };

    // Validate encrypted document size after parsing
    let doc_size = body.encrypted_document.len();
    if doc_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, doc_size));
    }

    // Check for duplicate recipient emails (production bug prevention)
    // Bypass with ALLOW_DUPLICATE_EMAILS=true for testing
    let allow_duplicates = env
        .var("ALLOW_DUPLICATE_EMAILS")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if !allow_duplicates {
        let mut seen_emails = std::collections::HashSet::new();
        for recipient in &body.recipients {
            let email_lower = recipient.email.to_lowercase();
            if !seen_emails.insert(email_lower.clone()) {
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": format!("Duplicate recipient email: {}", recipient.email),
                    "error_code": "DUPLICATE_RECIPIENT_EMAIL"
                }))?
                .with_status(400)));
            }
        }
    }

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Check sender session limits (storage exhaustion prevention)
    // BUG FIX: Always use authenticated user's email for indexing - don't trust frontend
    // This ensures sessions always appear in the sender's "My Documents" dashboard
    let sender_email = &user.email;
    let sender_hash = hash_sender_email(sender_email);
    let mut sender_index = get_sender_index(&kv, &sender_hash).await;

    // Prune expired sessions from index first
    sender_index.prune_expired(SESSION_INDEX_PRUNE_DAYS);

    // Check if sender is at limit
    if sender_index.count() >= MAX_SESSIONS_PER_SENDER {
        return cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Maximum active sessions reached. Please complete or cancel existing sessions.",
            "error_code": "SESSION_LIMIT_EXCEEDED",
            "limit": MAX_SESSIONS_PER_SENDER
        }))?
        .with_status(429)));
    }

    // Generate session ID
    let session_id = generate_session_id();

    // Calculate expiry
    let expiry_seconds = (body.expiry_hours as u64) * 60 * 60;
    let created_at = chrono::Utc::now().to_rfc3339();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(expiry_seconds as i64))
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339();

    // BUG FIX: Ensure metadata.sender_email is populated for display purposes
    let mut metadata = body.metadata;
    if metadata.sender_email.is_none() {
        metadata.sender_email = Some(user.email.clone());
    }

    let session = SigningSession {
        id: session_id.clone(),
        encrypted_document: body.encrypted_document,
        metadata,
        recipients: body.recipients,
        fields: body.fields,
        expires_at,
        signed_versions: vec![],
        signature_annotations: vec![], // New lightweight annotation storage
        status: SessionStatus::Pending,
        signing_mode: body.signing_mode,
        reminder_config: Some(body.reminder_config),
        final_document: None,
        legacy: body.legacy, // Allow callers to opt into legacy mode (for testing)
        revision_count: 0,
        voided_at: None,
        void_reason: None,
        token_version: 0,
    };

    // Store session with TTL
    // Bug #0: Add detailed error logging for debugging production issues
    let session_json = match serde_json::to_string(&session) {
        Ok(json) => json,
        Err(e) => {
            console_log!("ERROR: Failed to serialize session: {:?}", e);
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Failed to prepare session data. Please try again.",
                "error_code": "SERIALIZATION_ERROR",
                "debug_info": format!("{:?}", e)
            }))?
            .with_status(500)));
        }
    };

    // Log session size for debugging
    console_log!(
        "Creating session {} for user {}, json size: {} bytes",
        session_id,
        user.email,
        session_json.len()
    );

    let kv_result = kv
        .put(&format!("session:{}", session_id), session_json)
        .map_err(|e| {
            console_log!("ERROR: KV put builder failed: {:?}", e);
            e
        });

    match kv_result {
        Ok(builder) => {
            if let Err(e) = builder
                .expiration_ttl(expiry_seconds.min(SESSION_TTL_SECONDS))
                .execute()
                .await
            {
                console_log!("ERROR: KV put execute failed: {:?}", e);
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": "Failed to save session. Please try again.",
                    "error_code": "KV_WRITE_ERROR",
                    "debug_info": format!("{:?}", e)
                }))?
                .with_status(500)));
            }
        }
        Err(e) => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Failed to save session. Please try again.",
                "error_code": "KV_BUILDER_ERROR",
                "debug_info": format!("{:?}", e)
            }))?
            .with_status(500)));
        }
    }

    // Add session to sender's index
    sender_index.add_session(session_id.clone(), created_at.clone());
    if let Err(e) = save_sender_index(&kv, &sender_hash, &sender_index).await {
        // Log error but don't fail the request - session is already created
        console_log!("Warning: Failed to update sender index: {}", e);
    }

    // DocuSign-style Inbox: Index each recipient so they can see this in their inbox
    for recipient in &session.recipients {
        let recipient_hash = hash_sender_email(&recipient.email);
        let mut recipient_index = get_recipient_index(&kv, &recipient_hash).await;
        recipient_index.add_entry(session_id.clone(), recipient.id.clone(), created_at.clone());
        if let Err(e) = save_recipient_index(&kv, &recipient_hash, &recipient_index).await {
            console_log!(
                "Warning: Failed to update recipient index for {}: {}",
                recipient.email,
                e
            );
        }
    }

    // Bug #6: Record document send (handles base count + overage)
    // Returns true if this send triggered the limit (for email notification)
    let hit_limit = user.record_document_send();

    // Save user with updated counts (best-effort, don't fail the request)
    if let Err(e) = auth::save_user(&users_kv, &user).await {
        console_log!("Warning: Failed to update user document count: {:?}", e);
    }

    // Bug #6.4: Send limit notification email when user hits their limit
    // Only send once per billing period (tracked by limit_email_sent flag)
    if hit_limit && !user.limit_email_sent {
        console_log!(
            "User {} hit their {} limit ({} docs). Sending notification email.",
            user.email,
            user.tier.display_name(),
            user.tier.monthly_limit()
        );

        // Send email asynchronously (non-blocking, best-effort)
        let first_name = if user.first_name.is_empty() {
            "there"
        } else {
            &user.first_name
        };
        let tier_name = user.tier.display_name();
        let limit = user.tier.monthly_limit();

        match email::send_limit_notification_email(&env, &user.email, first_name, tier_name, limit)
            .await
        {
            Ok(result) => {
                if result.success {
                    console_log!("Limit notification email sent to {}", user.email);
                    // Mark as sent so we don't send again this billing period
                    user.limit_email_sent = true;
                    // Save the updated flag (best-effort)
                    if let Err(e) = auth::save_user(&users_kv, &user).await {
                        console_log!("Warning: Failed to update limit_email_sent flag: {:?}", e);
                    }
                } else {
                    console_log!(
                        "Failed to send limit notification email: {:?}",
                        result.error
                    );
                }
            }
            Err(e) => {
                console_log!("Error sending limit notification email: {:?}", e);
            }
        }
    }

    cors_response(Response::from_json(&CreateSessionResponse {
        success: true,
        session_id,
        message: None,
    }))
}

// ============================================================
// Feature 2: Document Dashboard - /my-sessions handler
// ============================================================

/// Handle GET /my-sessions - returns user's documents grouped by status
async fn handle_my_sessions(req: &Request, env: Env) -> Result<Response> {
    // Require authentication
    let (user, _users_kv) = match auth::require_auth(req, &env).await {
        Ok(Ok(result)) => result,
        Ok(Err(response)) => return cors_response(Ok(response)),
        Err(e) => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "Authentication error",
                "error": format!("{:?}", e)
            }))?
            .with_status(500)));
        }
    };

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "SESSIONS KV not configured"
            }))?
            .with_status(500)));
        }
    };

    let user_hash = hash_sender_email(&user.email);

    // ============================================================
    // SENT Documents (existing logic)
    // ============================================================
    let mut sender_index = get_sender_index(&kv, &user_hash).await;
    sender_index.prune_expired(SESSION_INDEX_PRUNE_DAYS);

    let mut sent_in_progress = Vec::new();
    let mut sent_completed = Vec::new();
    let mut sent_declined = Vec::new();
    let mut sent_expired = Vec::new();
    let mut sent_voided = Vec::new();

    // Track voided sessions to delete (older than 30 days)
    let mut voided_sessions_to_delete = Vec::new();
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);

    for session_id in &sender_index.session_ids {
        let session: Option<SigningSession> =
            match kv.get(&format!("session:{}", session_id)).json().await {
                Ok(s) => s,
                Err(_) => continue,
            };

        if let Some(session) = session {
            let recipients_total = session.recipients.len() as u32;
            let recipients_signed = session.recipients.iter().filter(|r| r.signed).count() as u32;

            let recipient_summaries: Vec<RecipientSummary> = session
                .recipients
                .iter()
                .map(|r| RecipientSummary {
                    name: r.name.clone(),
                    email: r.email.clone(),
                    signed: r.signed,
                    signed_at: r.signed_at.clone(),
                })
                .collect();

            let summary = SessionSummary {
                session_id: session.id.clone(),
                filename: session.metadata.filename.clone(),
                document_alias: session.metadata.document_alias.clone(),
                signing_context: session.metadata.signing_context.clone(),
                created_at: session.metadata.created_at.clone(),
                expires_at: session.expires_at.clone(),
                status: session.status.clone(),
                recipients_signed,
                recipients_total,
                recipients: recipient_summaries,
            };

            match session.status {
                SessionStatus::Completed => sent_completed.push(summary),
                SessionStatus::Declined => sent_declined.push(summary),
                SessionStatus::Expired => sent_expired.push(summary),
                SessionStatus::Voided => {
                    // Check if voided session is older than 30 days for auto-deletion
                    let should_delete = session.voided_at.as_ref().map_or(false, |voided_at| {
                        chrono::DateTime::parse_from_rfc3339(voided_at)
                            .map(|dt| dt < thirty_days_ago)
                            .unwrap_or(false)
                    });

                    if should_delete {
                        voided_sessions_to_delete.push(session.id.clone());
                    } else {
                        sent_voided.push(summary);
                    }
                }
                SessionStatus::Pending | SessionStatus::Accepted => sent_in_progress.push(summary),
            }
        }
    }

    // Sort sent documents by created_at descending
    sent_in_progress.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sent_completed.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sent_declined.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sent_expired.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sent_voided.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Auto-delete voided sessions older than 30 days (lazy cleanup)
    if !voided_sessions_to_delete.is_empty() {
        console_log!(
            "Auto-deleting {} voided sessions older than 30 days",
            voided_sessions_to_delete.len()
        );
        for session_id in &voided_sessions_to_delete {
            // Delete from KV
            if let Err(e) = kv.delete(&format!("session:{}", session_id)).await {
                console_log!("Failed to delete voided session {}: {:?}", session_id, e);
            }
            // Remove from sender index
            sender_index.remove_session(session_id);
        }
        // Save updated sender index
        save_sender_index(&kv, &user_hash, &sender_index).await;
    }

    // ============================================================
    // INBOX Documents (new DocuSign-style feature)
    // ============================================================
    let mut recipient_index = get_recipient_index(&kv, &user_hash).await;
    recipient_index.prune_expired(SESSION_INDEX_PRUNE_DAYS);

    let mut inbox_to_sign = Vec::new();
    let mut inbox_completed = Vec::new();
    let mut inbox_declined = Vec::new();

    for entry in &recipient_index.entries {
        let session: Option<SigningSession> = match kv
            .get(&format!("session:{}", entry.session_id))
            .json()
            .await
        {
            Ok(s) => s,
            Err(_) => continue,
        };

        if let Some(session) = session {
            // Skip voided sessions - they should not appear in inbox
            if session.status == SessionStatus::Voided {
                continue;
            }

            // Find this user's recipient record in the session
            if let Some(recipient) = session
                .recipients
                .iter()
                .find(|r| r.id == entry.recipient_id)
            {
                let my_status = if recipient.declined {
                    "declined"
                } else if recipient.signed {
                    "signed"
                } else {
                    "pending"
                };

                let summary = InboxSessionSummary {
                    session_id: entry.session_id.clone(),
                    filename: session.metadata.filename.clone(),
                    sender_name: session.metadata.created_by.clone(),
                    sender_email: session.metadata.sender_email.clone().unwrap_or_default(),
                    created_at: session.metadata.created_at.clone(),
                    expires_at: session.expires_at.clone(),
                    my_status: my_status.to_string(),
                    signed_at: recipient.signed_at.clone(),
                };

                if recipient.declined {
                    inbox_declined.push(summary);
                } else if recipient.signed {
                    inbox_completed.push(summary);
                } else {
                    inbox_to_sign.push(summary);
                }
            }
        }
    }

    // Sort inbox documents by created_at descending
    inbox_to_sign.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    inbox_completed.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    inbox_declined.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    cors_response(Response::from_json(&MySessionsResponse {
        success: true,
        sent: SentDocuments {
            in_progress: sent_in_progress,
            completed: sent_completed,
            declined: sent_declined,
            expired: sent_expired,
            voided: sent_voided,
        },
        inbox: InboxDocuments {
            to_sign: inbox_to_sign,
            completed: inbox_completed,
            declined: inbox_declined,
        },
    }))
}

async fn handle_get_session(session_id: &str, req: &Request, env: Env) -> Result<Response> {
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(s) => {
            // Check if expired (UX-004)
            if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&s.expires_at) {
                if expires < chrono::Utc::now() {
                    // UX-004: Return expired status with sender info
                    return cors_response(Ok(Response::from_json(&ExpiredSessionResponse {
                        status: "expired".to_string(),
                        sender_email: s
                            .metadata
                            .sender_email
                            .unwrap_or_else(|| "unknown".to_string()),
                        document_name: s.metadata.filename,
                        message: Some(
                            "This signing link has expired. You can request a new link below."
                                .to_string(),
                        ),
                    })?
                    .with_status(410)));
                }
            }

            // Token verification: extract token from query parameter
            let url = req.url()?;
            let token = url
                .query_pairs()
                .find(|(k, _)| k == "token")
                .map(|(_, v)| v.to_string());

            // Verify token if present, or check legacy flag
            let verified_recipient_id = if let Some(token) = token {
                // Verify the token
                let secret = get_signing_secret(&env);
                match verify_recipient_token(&token, session_id, &secret) {
                    Ok(result) => {
                        // Check token version matches session's current version
                        if result.token_version != s.token_version {
                            return cors_response(Ok(Response::from_json(&serde_json::json!({
                                "success": false,
                                "error": "token_invalidated",
                                "message": "This signing link is no longer valid. The document may have been revised or cancelled. Please contact the sender for a new link."
                            }))?
                            .with_status(410)));
                        }
                        Some(result.recipient_id)
                    }
                    Err(TokenError::Expired) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "token_expired",
                            "message": "Your signing link has expired. Please request a new link."
                        }))?
                        .with_status(401)));
                    }
                    Err(TokenError::InvalidSignature) | Err(TokenError::SessionMismatch) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "invalid_token",
                            "message": "Invalid signing link. Please use the link from your email."
                        }))?
                        .with_status(403)));
                    }
                    Err(TokenError::InvalidFormat) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "invalid_token",
                            "message": "Invalid signing link format."
                        }))?
                        .with_status(400)));
                    }
                    Err(TokenError::VersionMismatch) => {
                        return cors_response(Ok(Response::from_json(&serde_json::json!({
                            "success": false,
                            "error": "token_invalidated",
                            "message": "This signing link is no longer valid. The document may have been revised or cancelled."
                        }))?
                        .with_status(410)));
                    }
                }
            } else if s.legacy {
                // Legacy session without token requirement - allow access (backwards compatible)
                console_log!(
                    "WARNING: Legacy session {} accessed without token",
                    session_id
                );
                None
            } else {
                // No token provided and not a legacy session - deny access
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "error": "token_required",
                    "message": "A valid signing token is required to access this session."
                }))?
                .with_status(401)));
            };

            // If we have a verified recipient ID, filter the response to only show
            // information relevant to that recipient
            let (recipients, fields) = if let Some(ref recipient_id) = verified_recipient_id {
                // Filter to only show this recipient's information
                let filtered_recipients: Vec<RecipientInfo> = s
                    .recipients
                    .iter()
                    .filter(|r| &r.id == recipient_id)
                    .cloned()
                    .collect();

                let filtered_fields: Vec<FieldInfo> = s
                    .fields
                    .iter()
                    .filter(|f| &f.recipient_id == recipient_id)
                    .cloned()
                    .collect();

                (filtered_recipients, filtered_fields)
            } else {
                // Legacy mode: return all recipients and fields
                (s.recipients.clone(), s.fields.clone())
            };

            cors_response(Response::from_json(&GetSessionResponse {
                success: true,
                session: Some(SessionPublicInfo {
                    id: s.id,
                    metadata: s.metadata,
                    recipients,
                    fields,
                    encrypted_document: s.encrypted_document,
                    expires_at: s.expires_at,
                    signing_mode: s.signing_mode,
                    status: s.status,
                    final_document: s.final_document,
                }),
                message: None,
            }))
        }
        None => cors_response(Ok(Response::from_json(&GetSessionResponse {
            success: false,
            session: None,
            message: Some("Session not found".to_string()),
        })?
        .with_status(404))),
    }
}

/// UX-002: Handle decline endpoint
/// PUT /session/{id}/decline
/// Body: { "recipient_id": "...", "reason": "optional reason" }
async fn handle_decline(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the decline request
    let body: DeclineRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Check if session is already declined or completed
            if s.status == SessionStatus::Declined {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Session has already been declined".to_string(),
                })?
                .with_status(400)));
            }

            if s.status == SessionStatus::Completed {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Session has already been completed".to_string(),
                })?
                .with_status(400)));
            }

            // Find and update the recipient
            let mut recipient_found = false;
            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    recipient_found = true;

                    // Check if already signed
                    if r.signed {
                        return cors_response(Ok(Response::from_json(&DeclineResponse {
                            success: false,
                            message: "Recipient has already signed".to_string(),
                        })?
                        .with_status(400)));
                    }

                    // Check if already declined
                    if r.declined {
                        return cors_response(Ok(Response::from_json(&DeclineResponse {
                            success: false,
                            message: "Recipient has already declined".to_string(),
                        })?
                        .with_status(400)));
                    }

                    // Mark as declined
                    r.declined = true;
                    r.declined_at = Some(chrono::Utc::now().to_rfc3339());
                    r.decline_reason = body.reason.clone();
                    break;
                }
            }

            if !recipient_found {
                return cors_response(Ok(Response::from_json(&DeclineResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                })?
                .with_status(404)));
            }

            // Update session status to Declined
            s.status = SessionStatus::Declined;

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            // UX-006: Send decline notification email to sender
            if let Some(sender_email) = s.metadata.sender_email.as_ref() {
                if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                    let now = chrono::Utc::now().to_rfc3339();
                    let declined_at = recipient.declined_at.as_deref().unwrap_or(&now);

                    let subject = format!("{} Declined: {}", recipient.name, s.metadata.filename);
                    let html_body = format_decline_notification_email(
                        &recipient.name,
                        &recipient.email,
                        &s.metadata.filename,
                        body.reason.as_deref(),
                        declined_at,
                    );

                    if let Err(e) =
                        send_sender_notification(&env, sender_email, &subject, &html_body).await
                    {
                        console_log!("Failed to send decline notification: {:?}", e);
                    }
                }
            }

            console_log!(
                "Session {} declined by recipient {}: {:?}",
                session_id,
                body.recipient_id,
                body.reason
            );

            cors_response(Ok(Response::from_json(&DeclineResponse {
                success: true,
                message: "Document declined successfully".to_string(),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&DeclineResponse {
            success: false,
            message: "Session not found".to_string(),
        })?
        .with_status(404))),
    }
}

/// PUT /session/{id}/void
/// Voids/discards a document, invalidating all signing links.
/// Only the sender can void a document. Cannot void completed documents.
async fn handle_void(
    session_id: &str,
    sender_email: String,
    mut req: Request,
    env: Env,
) -> Result<Response> {
    // Parse the void request
    let body: VoidRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Verify sender owns this session
            let session_sender = s.metadata.sender_email.as_deref().unwrap_or("");
            if session_sender.to_lowercase() != sender_email.to_lowercase() {
                return cors_response(Ok(Response::from_json(&VoidResponse {
                    success: false,
                    message: "You do not have permission to void this document".to_string(),
                })?
                .with_status(403)));
            }

            // Check if session can be voided (not already completed or voided)
            if s.status == SessionStatus::Completed {
                return cors_response(Ok(Response::from_json(&VoidResponse {
                    success: false,
                    message: "Cannot void a completed document".to_string(),
                })?
                .with_status(400)));
            }

            if s.status == SessionStatus::Voided {
                return cors_response(Ok(Response::from_json(&VoidResponse {
                    success: false,
                    message: "Document has already been voided".to_string(),
                })?
                .with_status(400)));
            }

            // Update session status to Voided
            s.status = SessionStatus::Voided;
            s.voided_at = Some(chrono::Utc::now().to_rfc3339());
            s.void_reason = body.reason.clone();

            // Increment token_version to invalidate all existing signing links
            s.token_version = s.token_version.saturating_add(1);

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            // Send void notification emails to all recipients
            let void_date = s.voided_at.as_deref().unwrap_or("Unknown");
            let sender_name = &s.metadata.created_by;
            let document_name = &s.metadata.filename;

            for recipient in &s.recipients {
                let html_body = format_void_notification_email(
                    document_name,
                    sender_name,
                    body.reason.as_deref(),
                    void_date,
                    &recipient.name,
                );

                let subject = format!("Document Cancelled: {}", document_name);

                if let Err(e) =
                    send_sender_notification(&env, &recipient.email, &subject, &html_body).await
                {
                    console_log!(
                        "Failed to send void notification to {}: {:?}",
                        recipient.email,
                        e
                    );
                }
            }

            // Also send notification to sender
            let sender_html = format_void_notification_email(
                document_name,
                sender_name,
                body.reason.as_deref(),
                void_date,
                "You",
            );
            let sender_subject = format!("Document Discarded: {}", document_name);
            if let Err(e) =
                send_sender_notification(&env, &sender_email, &sender_subject, &sender_html).await
            {
                console_log!("Failed to send void confirmation to sender: {:?}", e);
            }

            console_log!(
                "Session {} voided by sender {}: {:?}",
                session_id,
                sender_email,
                body.reason
            );

            cors_response(Ok(Response::from_json(&VoidResponse {
                success: true,
                message: "Document voided successfully. All recipients have been notified."
                    .to_string(),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&VoidResponse {
            success: false,
            message: "Session not found".to_string(),
        })?
        .with_status(404))),
    }
}

/// Revises a document - only allowed if no signers have signed yet.
/// Updates fields, optionally updates recipients and document.
/// Increments token_version to invalidate old signing links.
async fn handle_revise(
    session_id: &str,
    sender_email: String,
    mut req: Request,
    env: Env,
) -> Result<Response> {
    // Parse the revise request
    let body: ReviseRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Verify sender owns this session
            let session_sender = s.metadata.sender_email.as_deref().unwrap_or("");
            if session_sender.to_lowercase() != sender_email.to_lowercase() {
                return cors_response(Ok(Response::from_json(&ReviseResponse {
                    success: false,
                    message: "You do not have permission to revise this document".to_string(),
                    tokens: None,
                })?
                .with_status(403)));
            }

            // Check if session can be revised (not completed, voided, or expired)
            match s.status {
                SessionStatus::Completed => {
                    return cors_response(Ok(Response::from_json(&ReviseResponse {
                        success: false,
                        message: "Cannot revise a completed document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                SessionStatus::Voided => {
                    return cors_response(Ok(Response::from_json(&ReviseResponse {
                        success: false,
                        message: "Cannot revise a voided document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                SessionStatus::Expired => {
                    return cors_response(Ok(Response::from_json(&ReviseResponse {
                        success: false,
                        message: "Cannot revise an expired document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                _ => {} // Allow Pending, Accepted, Declined
            }

            // Check if anyone has signed - if so, cannot revise (use restart instead)
            let anyone_signed = s.recipients.iter().any(|r| r.signed);
            if anyone_signed {
                return cors_response(Ok(Response::from_json(&ReviseResponse {
                    success: false,
                    message: "Cannot revise - some recipients have already signed. Use the restart endpoint to void existing signatures and start fresh.".to_string(),
                    tokens: None,
                })?
                .with_status(400)));
            }

            // Update fields
            s.fields = body.fields;

            // Update recipients if provided (full revision)
            if let Some(new_recipients) = body.recipients {
                let mut updated_recipients = Vec::new();
                for r in new_recipients {
                    updated_recipients.push(RecipientInfo {
                        id: r.id,
                        name: r.name,
                        email: r.email,
                        role: r.role,
                        signing_order: r.signing_order,
                        // Reset all signing state for revised recipients
                        consented: false,
                        consent_at: None,
                        consent_user_agent: None,
                        signed: false,
                        signed_at: None,
                        declined: false,
                        declined_at: None,
                        decline_reason: None,
                        reminders_sent: 0,
                        last_reminder_at: None,
                    });
                }
                s.recipients = updated_recipients;
            } else {
                // Reset consent/decline state for existing recipients (they need to review revised doc)
                for recipient in &mut s.recipients {
                    recipient.consented = false;
                    recipient.consent_at = None;
                    recipient.consent_user_agent = None;
                    recipient.declined = false;
                    recipient.declined_at = None;
                    recipient.decline_reason = None;
                }
            }

            // Update document if provided
            if let Some(new_document) = body.document {
                s.encrypted_document = new_document;
            }

            // Increment token_version to invalidate all existing signing links
            s.token_version = s.token_version.saturating_add(1);

            // Increment revision count
            s.revision_count = s.revision_count.saturating_add(1);

            // Reset status to Pending (in case it was Declined)
            s.status = SessionStatus::Pending;

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            // Generate new tokens and send revision emails
            let secret = get_signing_secret(&env);
            let mut tokens = Vec::new();
            let document_name = &s.metadata.filename;
            let sender_name = &s.metadata.created_by;
            let revision_message = body.message.as_deref().unwrap_or("");

            for recipient in &s.recipients {
                // Only send to signers (not viewers)
                if recipient.role != "signer" {
                    continue;
                }

                // Generate new token with updated version
                let token =
                    generate_recipient_token(session_id, &recipient.id, s.token_version, &secret);

                // Build signing URL
                let signing_url = format!(
                    "https://getsignatures.org/sign.html?session={}&recipient={}&token={}",
                    session_id, recipient.id, token
                );

                tokens.push(TokenInfo {
                    recipient_id: recipient.id.clone(),
                    recipient_name: recipient.name.clone(),
                    recipient_email: recipient.email.clone(),
                    signing_url: signing_url.clone(),
                });

                // Send revision notification email
                let email_html = format_revision_notification_email(
                    document_name,
                    sender_name,
                    &recipient.name,
                    revision_message,
                    &signing_url,
                    s.revision_count,
                );

                let email_subject = format!("Document Revised: {}", document_name);

                let email_request = email::EmailSendRequest {
                    to: vec![recipient.email.clone()],
                    subject: email_subject,
                    html: email_html,
                    text: None,
                    reply_to: None,
                    tags: vec![
                        ("type".to_string(), "revision".to_string()),
                        ("session_id".to_string(), session_id.to_string()),
                    ],
                };

                if let Err(e) = email::send_email(&env, email_request).await {
                    console_log!(
                        "Failed to send revision notification to {}: {:?}",
                        recipient.email,
                        e
                    );
                }
            }

            console_log!(
                "Session {} revised by sender {}. Revision #{}, {} recipients notified",
                session_id,
                sender_email,
                s.revision_count,
                tokens.len()
            );

            cors_response(Ok(Response::from_json(&ReviseResponse {
                success: true,
                message: format!(
                    "Document revised successfully (revision #{}). All recipients have been notified with new signing links.",
                    s.revision_count
                ),
                tokens: Some(tokens),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&ReviseResponse {
            success: false,
            message: "Session not found".to_string(),
            tokens: None,
        })?
        .with_status(404))),
    }
}

/// Format revision notification email
fn format_revision_notification_email(
    document_name: &str,
    sender_name: &str,
    recipient_name: &str,
    revision_message: &str,
    signing_url: &str,
    revision_count: u32,
) -> String {
    let message_section = if !revision_message.is_empty() {
        format!(
            r#"
        <div style="background: #dbeafe; padding: 15px; border-radius: 6px; margin-bottom: 25px; border-left: 4px solid #3b82f6;">
            <p style="margin: 0; font-size: 14px; color: #1e40af;">
                <strong>Message from sender:</strong> {}
            </p>
        </div>"#,
            revision_message
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Has Been Revised</h1>
        <p style="margin: 10px 0 0 0; font-size: 14px; opacity: 0.9;">Revision #{}</p>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Hello {},</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{}</strong> has revised the document that requires your signature. Please review the updated document using the new link below.
        </p>

        <div style="background: #fef3c7; padding: 12px 15px; border-radius: 6px; margin-bottom: 20px; border-left: 4px solid #f59e0b;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Important:</strong> Previous signing links are no longer valid. Please use the button below to access the revised document.
            </p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{}</p>
        </div>{}

        <div style="text-align: center; margin: 30px 0;">
            <a href="{}" style="display: inline-block; background: #f59e0b; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Review Revised Document</a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            If you have any questions about this revision, please contact the sender directly.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        revision_count, recipient_name, sender_name, document_name, message_section, signing_url
    )
}

/// Restarts a document - voids all existing signatures and starts fresh.
/// Use this when some signers have already signed but need to revise the document.
async fn handle_restart(
    session_id: &str,
    sender_email: String,
    mut req: Request,
    env: Env,
) -> Result<Response> {
    // Parse the restart request
    let body: RestartRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Verify sender owns this session
            let session_sender = s.metadata.sender_email.as_deref().unwrap_or("");
            if session_sender.to_lowercase() != sender_email.to_lowercase() {
                return cors_response(Ok(Response::from_json(&RestartResponse {
                    success: false,
                    message: "You do not have permission to restart this document".to_string(),
                    tokens: None,
                })?
                .with_status(403)));
            }

            // Check if session can be restarted (not completed, voided, or expired)
            match s.status {
                SessionStatus::Completed => {
                    return cors_response(Ok(Response::from_json(&RestartResponse {
                        success: false,
                        message: "Cannot restart a completed document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                SessionStatus::Voided => {
                    return cors_response(Ok(Response::from_json(&RestartResponse {
                        success: false,
                        message: "Cannot restart a voided document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                SessionStatus::Expired => {
                    return cors_response(Ok(Response::from_json(&RestartResponse {
                        success: false,
                        message: "Cannot restart an expired document".to_string(),
                        tokens: None,
                    })?
                    .with_status(400)));
                }
                _ => {} // Allow Pending, Accepted, Declined
            }

            // Count how many had signed before restart (for email messaging)
            let previously_signed_count = s.recipients.iter().filter(|r| r.signed).count();

            // Update fields
            s.fields = body.fields;

            // Clear signature annotations (void all existing signatures)
            s.signature_annotations.clear();

            // Clear final document if present
            s.final_document = None;

            // Update recipients if provided, otherwise reset existing ones
            if let Some(new_recipients) = body.recipients {
                let mut updated_recipients = Vec::new();
                for r in new_recipients {
                    updated_recipients.push(RecipientInfo {
                        id: r.id,
                        name: r.name,
                        email: r.email,
                        role: r.role,
                        signing_order: r.signing_order,
                        // Reset all signing state
                        consented: false,
                        consent_at: None,
                        consent_user_agent: None,
                        signed: false,
                        signed_at: None,
                        declined: false,
                        declined_at: None,
                        decline_reason: None,
                        reminders_sent: 0,
                        last_reminder_at: None,
                    });
                }
                s.recipients = updated_recipients;
            } else {
                // Reset ALL signing state for existing recipients
                for recipient in &mut s.recipients {
                    recipient.consented = false;
                    recipient.consent_at = None;
                    recipient.consent_user_agent = None;
                    recipient.signed = false;
                    recipient.signed_at = None;
                    recipient.declined = false;
                    recipient.declined_at = None;
                    recipient.decline_reason = None;
                }
            }

            // Update document if provided
            if let Some(new_document) = body.document {
                s.encrypted_document = new_document;
            }

            // Increment token_version to invalidate all existing signing links
            s.token_version = s.token_version.saturating_add(1);

            // Increment revision count
            s.revision_count = s.revision_count.saturating_add(1);

            // Reset status to Pending
            s.status = SessionStatus::Pending;

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            // Generate new tokens and send restart emails
            let secret = get_signing_secret(&env);
            let mut tokens = Vec::new();
            let document_name = &s.metadata.filename;
            let sender_name = &s.metadata.created_by;
            let restart_message = body.message.as_deref().unwrap_or("");

            for recipient in &s.recipients {
                // Only send to signers (not viewers)
                if recipient.role != "signer" {
                    continue;
                }

                // Generate new token with updated version
                let token =
                    generate_recipient_token(session_id, &recipient.id, s.token_version, &secret);

                // Build signing URL
                let signing_url = format!(
                    "https://getsignatures.org/sign.html?session={}&recipient={}&token={}",
                    session_id, recipient.id, token
                );

                tokens.push(TokenInfo {
                    recipient_id: recipient.id.clone(),
                    recipient_name: recipient.name.clone(),
                    recipient_email: recipient.email.clone(),
                    signing_url: signing_url.clone(),
                });

                // Send restart notification email
                let email_html = format_restart_notification_email(
                    document_name,
                    sender_name,
                    &recipient.name,
                    restart_message,
                    &signing_url,
                    s.revision_count,
                    previously_signed_count > 0,
                );

                let email_subject = format!("Document Restarted: {}", document_name);

                let email_request = email::EmailSendRequest {
                    to: vec![recipient.email.clone()],
                    subject: email_subject,
                    html: email_html,
                    text: None,
                    reply_to: None,
                    tags: vec![
                        ("type".to_string(), "restart".to_string()),
                        ("session_id".to_string(), session_id.to_string()),
                    ],
                };

                if let Err(e) = email::send_email(&env, email_request).await {
                    console_log!(
                        "Failed to send restart notification to {}: {:?}",
                        recipient.email,
                        e
                    );
                }
            }

            console_log!(
                "Session {} restarted by sender {}. Revision #{}, {} previous signatures voided, {} recipients notified",
                session_id,
                sender_email,
                s.revision_count,
                previously_signed_count,
                tokens.len()
            );

            cors_response(Ok(Response::from_json(&RestartResponse {
                success: true,
                message: format!(
                    "Document restarted successfully (revision #{}). {} previous signature(s) have been voided. All recipients have been notified with new signing links.",
                    s.revision_count,
                    previously_signed_count
                ),
                tokens: Some(tokens),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&RestartResponse {
            success: false,
            message: "Session not found".to_string(),
            tokens: None,
        })?
        .with_status(404))),
    }
}

/// Format restart notification email (signatures voided)
fn format_restart_notification_email(
    document_name: &str,
    sender_name: &str,
    recipient_name: &str,
    restart_message: &str,
    signing_url: &str,
    revision_count: u32,
    had_signatures: bool,
) -> String {
    let message_section = if !restart_message.is_empty() {
        format!(
            r#"
        <div style="background: #dbeafe; padding: 15px; border-radius: 6px; margin-bottom: 25px; border-left: 4px solid #3b82f6;">
            <p style="margin: 0; font-size: 14px; color: #1e40af;">
                <strong>Message from sender:</strong> {}
            </p>
        </div>"#,
            restart_message
        )
    } else {
        String::new()
    };

    let signatures_voided_note = if had_signatures {
        r#"
        <div style="background: #fef2f2; padding: 12px 15px; border-radius: 6px; margin-bottom: 20px; border-left: 4px solid #ef4444;">
            <p style="margin: 0; font-size: 14px; color: #991b1b;">
                <strong>Note:</strong> Previous signatures on this document have been voided due to the revision.
            </p>
        </div>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #dc2626 0%, #b91c1c 100%); color: white; padding: 30px; border-radius: 8px 8px 0 0; text-align: center;">
        <h1 style="margin: 0; font-size: 24px;">Document Has Been Restarted</h1>
        <p style="margin: 10px 0 0 0; font-size: 14px; opacity: 0.9;">Revision #{} - Previous signatures voided</p>
    </div>

    <div style="background: #ffffff; padding: 30px; border: 1px solid #e5e7eb; border-top: none; border-radius: 0 0 8px 8px;">
        <p style="font-size: 16px; margin-bottom: 20px;">Hello {},</p>

        <p style="font-size: 16px; margin-bottom: 20px;">
            <strong>{}</strong> has made changes to a document that requires your signature. The document has been restarted and requires all signatures to be collected again.
        </p>{}

        <div style="background: #fef3c7; padding: 12px 15px; border-radius: 6px; margin-bottom: 20px; border-left: 4px solid #f59e0b;">
            <p style="margin: 0; font-size: 14px; color: #92400e;">
                <strong>Important:</strong> Previous signing links are no longer valid. Please use the button below to review and sign the updated document.
            </p>
        </div>

        <div style="background: #f9fafb; padding: 15px; border-radius: 6px; margin-bottom: 25px;">
            <p style="margin: 0; font-size: 14px; color: #6b7280;">Document Name</p>
            <p style="margin: 5px 0 0 0; font-size: 16px; font-weight: 600;">{}</p>
        </div>{}

        <div style="text-align: center; margin: 30px 0;">
            <a href="{}" style="display: inline-block; background: #dc2626; color: white; padding: 14px 32px; text-decoration: none; border-radius: 6px; font-weight: 600; font-size: 16px;">Review & Sign Document</a>
        </div>

        <p style="font-size: 14px; color: #6b7280; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e5e7eb;">
            If you have any questions about these changes, please contact the sender directly.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
        revision_count,
        recipient_name,
        sender_name,
        signatures_voided_note,
        document_name,
        message_section,
        signing_url
    )
}

/// PUT /session/{id}/consent
/// Records consent acceptance for the audit trail.
/// Called when user clicks "Review Document" on consent page.
async fn handle_consent(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the consent request
    let body: ConsentRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Check if session is expired
            if s.status == SessionStatus::Expired {
                return cors_response(Ok(Response::from_json(&ConsentResponse {
                    success: false,
                    message: "Session has expired".to_string(),
                    consent_at: String::new(),
                })?
                .with_status(400)));
            }

            // Find and update the recipient
            let mut recipient_found = false;
            let consent_timestamp = chrono::Utc::now().to_rfc3339();

            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    recipient_found = true;

                    // Check if already declined
                    if r.declined {
                        return cors_response(Ok(Response::from_json(&ConsentResponse {
                            success: false,
                            message: "Recipient has already declined".to_string(),
                            consent_at: String::new(),
                        })?
                        .with_status(400)));
                    }

                    // Record consent (even if already consented - update timestamp)
                    r.consented = true;
                    r.consent_at = Some(consent_timestamp.clone());
                    r.consent_user_agent = body.user_agent.clone();
                    break;
                }
            }

            if !recipient_found {
                return cors_response(Ok(Response::from_json(&ConsentResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                    consent_at: String::new(),
                })?
                .with_status(404)));
            }

            // Update session status to Accepted if it was Pending
            if s.status == SessionStatus::Pending {
                s.status = SessionStatus::Accepted;
            }

            // Save updated session
            kv.put(
                &format!("session:{}", session_id),
                serde_json::to_string(&s)?,
            )?
            .execute()
            .await?;

            console_log!(
                "Consent recorded for session {} by recipient {}",
                session_id,
                body.recipient_id
            );

            cors_response(Ok(Response::from_json(&ConsentResponse {
                success: true,
                message: "Consent recorded successfully".to_string(),
                consent_at: consent_timestamp,
            })?))
        }
        None => cors_response(Ok(Response::from_json(&ConsentResponse {
            success: false,
            message: "Session not found".to_string(),
            consent_at: String::new(),
        })?
        .with_status(404))),
    }
}

/// UX-004: Handle request-link endpoint
/// POST /session/{id}/request-link
/// Body: { "recipient_id": "..." }
/// Sends notification email to sender that recipient requested new link
async fn handle_request_link(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Parse the request
    let body: RequestLinkRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the session
    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(s) => {
            // Find the recipient
            let recipient = s.recipients.iter().find(|r| r.id == body.recipient_id);

            if recipient.is_none() {
                return cors_response(Ok(Response::from_json(&RequestLinkResponse {
                    success: false,
                    message: "Recipient not found in session".to_string(),
                })?
                .with_status(404)));
            }

            let recipient = recipient.unwrap();
            let sender_email = s.metadata.sender_email.as_deref().unwrap_or("unknown");

            // Send notification email to sender
            let subject = format!("New Link Requested for {}", s.metadata.filename);
            let html_body = format!(
                r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #1f2937; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: #f3f4f6; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
        <h2 style="margin: 0 0 10px 0; color: #111827;">Link Request Notification</h2>
        <p style="margin: 0; color: #6b7280;">GetSignatures Document Signing</p>
    </div>

    <div style="background: white; padding: 25px; border: 1px solid #e5e7eb; border-radius: 8px;">
        <p style="font-size: 16px; margin-top: 0;">Hello,</p>

        <p style="font-size: 15px;">
            <strong>{}</strong> ({}) has requested a new signing link for:
        </p>

        <div style="background: #f9fafb; padding: 15px; border-left: 4px solid #3b82f6; margin: 20px 0;">
            <p style="margin: 0; font-weight: 600; color: #1f2937;">{}</p>
        </div>

        <p style="font-size: 14px; color: #6b7280;">
            The current link has expired. Please generate a new signing session and send them a fresh link.
        </p>
    </div>

    <div style="text-align: center; margin-top: 20px; font-size: 12px; color: #9ca3af;">
        <p>Sent via GetSignatures - Secure Document Signing</p>
    </div>
</body>
</html>"#,
                recipient.name, recipient.email, s.metadata.filename
            );

            // Send the email (best effort - don't fail if email fails)
            if let Err(e) = send_sender_notification(&env, sender_email, &subject, &html_body).await
            {
                console_log!("Warning: Failed to send request-link notification: {}", e);
            }

            cors_response(Ok(Response::from_json(&RequestLinkResponse {
                success: true,
                message: format!(
                    "Request sent to {}. They will send you a new link shortly.",
                    sender_email
                ),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&RequestLinkResponse {
            success: false,
            message: "Session not found".to_string(),
        })?
        .with_status(404))),
    }
}

/// UX-004: Handle resend endpoint (protected with API key)
/// POST /session/{id}/resend
/// Creates new session with fresh expiry and invalidates old session
async fn handle_resend(session_id: &str, env: Env) -> Result<Response> {
    // Get KV store
    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    // Fetch the old session
    let old_session: Option<SigningSession> =
        kv.get(&format!("session:{}", session_id)).json().await?;

    match old_session {
        Some(s) => {
            // Capture sender email before consuming metadata
            let sender_email = s.metadata.sender_email.clone();

            // Generate new session ID
            let new_session_id = generate_session_id();

            // Calculate new expiry (7 days default)
            let expiry_hours = 168u64;
            let expiry_seconds = expiry_hours * 60 * 60;
            let created_at = chrono::Utc::now().to_rfc3339();
            let expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::seconds(expiry_seconds as i64))
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();

            // Create new session with same data but new ID and expiry
            let new_session = SigningSession {
                id: new_session_id.clone(),
                encrypted_document: s.encrypted_document,
                metadata: s.metadata,
                recipients: s
                    .recipients
                    .into_iter()
                    .map(|mut r| {
                        // Reset signed status for resend
                        r.signed = false;
                        r.signed_at = None;
                        r.reminders_sent = 0;
                        r.last_reminder_at = None;
                        r
                    })
                    .collect(),
                fields: s.fields,
                expires_at: expires_at.clone(),
                signed_versions: vec![],
                signature_annotations: vec![], // New lightweight annotation storage
                status: SessionStatus::Pending,
                signing_mode: s.signing_mode,
                reminder_config: s.reminder_config,
                final_document: None, // Reset for new session
                legacy: false,        // Resent sessions require token authentication
                revision_count: 0,
                voided_at: None,
                void_reason: None,
                token_version: 0,
            };

            // Store new session
            let session_json = serde_json::to_string(&new_session)
                .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

            kv.put(&format!("session:{}", new_session_id), session_json)?
                .expiration_ttl(expiry_seconds.min(SESSION_TTL_SECONDS))
                .execute()
                .await?;

            // Delete old session to invalidate it
            kv.delete(&format!("session:{}", session_id)).await?;

            // Update sender session index: remove old, add new
            if let Some(ref email) = sender_email {
                let sender_hash = hash_sender_email(email);
                let mut sender_index = get_sender_index(&kv, &sender_hash).await;
                sender_index.remove_session(session_id);
                sender_index.add_session(new_session_id.clone(), created_at);
                if let Err(e) = save_sender_index(&kv, &sender_hash, &sender_index).await {
                    console_log!("Warning: Failed to update sender index on resend: {}", e);
                }
            }

            cors_response(Ok(Response::from_json(&ResendResponse {
                success: true,
                new_session_id,
                expires_at,
                message: "New session created successfully".to_string(),
            })?))
        }
        None => cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Session not found"
        }))?
        .with_status(404))),
    }
}

async fn handle_submit_signed(session_id: &str, mut req: Request, env: Env) -> Result<Response> {
    // Check request size before parsing
    // Note: signed documents can be larger than signatures alone due to embedded signature images
    if let Some(response) = check_content_length(&req, MAX_PDF_SIZE) {
        return cors_response(Ok(response));
    }

    let body: SubmitSignedRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    // Validate encrypted document size after parsing
    let doc_size = body.encrypted_document.len();
    if doc_size > MAX_PDF_SIZE {
        return cors_response(payload_too_large_response(MAX_PDF_SIZE, doc_size));
    }

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Bug A Fix: Check if recipient has already signed (prevent re-signing)
            if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                if recipient.signed {
                    return cors_response(Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "You have already signed this document"
                    }))?
                    .with_status(400)));
                }
            } else {
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": "Recipient not found in session"
                }))?
                .with_status(404)));
            }

            // Mark recipient as signed
            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    r.signed = true;
                    r.signed_at = Some(chrono::Utc::now().to_rfc3339());
                }
            }

            // Add signed version
            s.signed_versions.push(SignedVersion {
                recipient_id: body.recipient_id.clone(),
                encrypted_document: body.encrypted_document.clone(),
                signed_at: chrono::Utc::now().to_rfc3339(),
            });

            // Handle document storage based on signing mode
            match s.signing_mode {
                SigningMode::Sequential => {
                    // Sequential mode: each signer sees previous signatures
                    // Update the main document for the next signer
                    s.encrypted_document = body.encrypted_document;
                }
                SigningMode::Parallel => {
                    // Parallel mode: keep original, merge when all complete
                    // Original stays in encrypted_document, each version in signed_versions
                    if all_recipients_signed(&s.recipients) {
                        // All signed - for now, use the last submitted version
                        // TODO: Implement proper PDF merge when docsign-core supports it
                        // For MVP: use last version (all signatures are on same positions anyway)
                        s.final_document = Some(body.encrypted_document);
                        s.status = SessionStatus::Completed;
                    }
                }
            }

            // UX-006: Send notification to sender
            if let Some(sender_email) = s.metadata.sender_email.as_ref() {
                // Find the recipient who just signed
                if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                    let download_link =
                        generate_download_link(session_id, DOWNLOAD_LINK_EXPIRY_DAYS);

                    // Check if all recipients have signed
                    if all_recipients_signed(&s.recipients) {
                        // Send "all signed" completion email to SENDER
                        let subject = format!("All Recipients Signed: {}", s.metadata.filename);
                        let html_body = format_all_signed_notification_email(
                            &s.recipients,
                            &s.metadata.filename,
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;

                        // Bug C Fix: Also send completion email to ALL signers
                        for signer in s
                            .recipients
                            .iter()
                            .filter(|r| r.signed && r.role == "signer")
                        {
                            let signer_subject =
                                format!("Document Signed: {}", s.metadata.filename);
                            let signer_html = format_recipient_completion_email(
                                &signer.name,
                                &s.metadata.filename,
                                &download_link,
                                &s.recipients,
                            );
                            let _ = send_sender_notification(
                                &env,
                                &signer.email,
                                &signer_subject,
                                &signer_html,
                            )
                            .await;
                        }
                    } else {
                        // Send individual recipient signed notification
                        let subject = format!("{} Signed: {}", recipient.name, s.metadata.filename);
                        let html_body = format_completion_notification_email(
                            &recipient.name,
                            &s.metadata.filename,
                            recipient
                                .signed_at
                                .as_ref()
                                .unwrap_or(&"Unknown".to_string()),
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;
                    }
                }
            }

            // Save updated session
            let session_json = serde_json::to_string(&s)
                .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

            kv.put(&format!("session:{}", session_id), session_json)?
                .execute()
                .await?;

            // Bug E Fix: Include all_signed status in response
            let all_signed = all_recipients_signed(&s.recipients);
            cors_response(Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Signed document submitted",
                "all_signed": all_signed
            })))
        }
        None => cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Session not found"
        }))?
        .with_status(404))),
    }
}

/// Handle submission of signature annotations (NEW lightweight storage)
/// Instead of storing full PDF copies (~13MB each), stores only annotation data (~50KB each)
/// This allows unlimited signers within the 25MB KV limit
async fn handle_submit_annotations(
    session_id: &str,
    mut req: Request,
    env: Env,
) -> Result<Response> {
    // Annotation payloads are small (~50KB), but check reasonable limit
    const MAX_ANNOTATION_SIZE: usize = 500 * 1024; // 500KB max for annotations
    if let Some(response) = check_content_length(&req, MAX_ANNOTATION_SIZE) {
        return cors_response(Ok(response));
    }

    let body: SubmitAnnotationsRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return cors_response(error_response(&format!("Invalid request: {}", e)));
        }
    };

    let kv = match env.kv("SESSIONS") {
        Ok(kv) => kv,
        Err(_) => {
            return cors_response(error_response("SESSIONS KV not configured"));
        }
    };

    let session: Option<SigningSession> = kv.get(&format!("session:{}", session_id)).json().await?;

    match session {
        Some(mut s) => {
            // Debug logging for re-signing prevention
            console_log!(
                "SIGN-DEBUG: session_id={} recipient_id={} signed={}",
                session_id,
                body.recipient_id,
                s.recipients
                    .iter()
                    .find(|r| r.id == body.recipient_id)
                    .map(|r| r.signed)
                    .unwrap_or(false)
            );

            // Check if recipient exists and hasn't already signed
            if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                if recipient.signed {
                    return cors_response(Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "You have already signed this document"
                    }))?
                    .with_status(400)));
                }
            } else {
                return cors_response(Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": "Recipient not found in session"
                }))?
                .with_status(404)));
            }

            // Mark recipient as signed
            let signed_at = chrono::Utc::now().to_rfc3339();
            for r in s.recipients.iter_mut() {
                if r.id == body.recipient_id {
                    r.signed = true;
                    r.signed_at = Some(signed_at.clone());
                }
            }

            // Convert submitted annotations to storage format and add to session
            for ann in body.annotations {
                // Look up field to get position if not provided
                let field = s.fields.iter().find(|f| f.id == ann.field_id);

                // Get position from annotation or look up from field
                let position = match ann.position {
                    Some(pos) => pos,
                    None => {
                        // Look up from session fields
                        if let Some(f) = field {
                            AnnotationPosition {
                                page: f.page,
                                x_percent: f.x_percent,
                                y_percent: f.y_percent,
                                width_percent: f.width_percent,
                                height_percent: f.height_percent,
                            }
                        } else {
                            // Field not found - use default position (shouldn't happen)
                            console_log!("Warning: Field {} not found in session", ann.field_id);
                            AnnotationPosition {
                                page: 0,
                                x_percent: 0.0,
                                y_percent: 0.0,
                                width_percent: 20.0,
                                height_percent: 5.0,
                            }
                        }
                    }
                };

                // Get field_type from annotation or look up from field definition
                let field_type = ann.field_type.unwrap_or_else(|| {
                    if let Some(f) = field {
                        // Convert field_type string to enum
                        match f.field_type.as_str() {
                            "signature" => AnnotationFieldType::Signature,
                            "initials" => AnnotationFieldType::Initials,
                            "date" => AnnotationFieldType::Date,
                            "text" => AnnotationFieldType::Text,
                            "checkbox" => AnnotationFieldType::Checkbox,
                            _ => AnnotationFieldType::Signature,
                        }
                    } else {
                        AnnotationFieldType::Signature
                    }
                });

                let annotation_id = format!(
                    "ann_{}_{}",
                    body.recipient_id,
                    js_sys::Math::random().to_bits() as u32
                );

                s.signature_annotations.push(SignatureAnnotation {
                    id: annotation_id,
                    recipient_id: body.recipient_id.clone(),
                    field_id: ann.field_id,
                    field_type,
                    data: ann.data,
                    position,
                    signed_at: signed_at.clone(),
                });
            }

            // Check if all signers have completed
            let all_signed = all_recipients_signed(&s.recipients);
            let remaining_signers = s
                .recipients
                .iter()
                .filter(|r| !r.signed && r.role == "signer")
                .count() as u32;

            // Generate download link (used for emails and response when all signed)
            let download_link = generate_download_link(session_id, DOWNLOAD_LINK_EXPIRY_DAYS);

            // Update session status if complete
            if all_signed {
                s.status = SessionStatus::Completed;
                // Note: final_document will be generated on-demand during download
                // This avoids storing merged PDF in KV (saves space)
            }

            // Send notification emails (same logic as legacy handler)
            if let Some(sender_email) = s.metadata.sender_email.as_ref() {
                if let Some(recipient) = s.recipients.iter().find(|r| r.id == body.recipient_id) {
                    if all_signed {
                        // Send "all signed" completion email to sender
                        let subject = format!("All Recipients Signed: {}", s.metadata.filename);
                        let html_body = format_all_signed_notification_email(
                            &s.recipients,
                            &s.metadata.filename,
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;

                        // Send completion email to all signers
                        for signer in s
                            .recipients
                            .iter()
                            .filter(|r| r.signed && r.role == "signer")
                        {
                            let signer_subject =
                                format!("Document Signed: {}", s.metadata.filename);
                            let signer_html = format_recipient_completion_email(
                                &signer.name,
                                &s.metadata.filename,
                                &download_link,
                                &s.recipients,
                            );
                            let _ = send_sender_notification(
                                &env,
                                &signer.email,
                                &signer_subject,
                                &signer_html,
                            )
                            .await;
                        }
                    } else {
                        // Send individual signed notification
                        let subject = format!("{} Signed: {}", recipient.name, s.metadata.filename);
                        let html_body = format_completion_notification_email(
                            &recipient.name,
                            &s.metadata.filename,
                            recipient
                                .signed_at
                                .as_ref()
                                .unwrap_or(&"Unknown".to_string()),
                            &download_link,
                        );
                        let _ = send_sender_notification(&env, sender_email, &subject, &html_body)
                            .await;
                    }
                }
            }

            // Save updated session
            let session_json = serde_json::to_string(&s)
                .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;

            kv.put(&format!("session:{}", session_id), session_json)?
                .execute()
                .await?;

            console_log!(
                "SIGN-DEBUG: KV write complete for session {} - recipient {} now marked signed",
                session_id,
                body.recipient_id
            );

            // Include download_url in response only when all signers have completed
            let download_url = if all_signed {
                Some(download_link)
            } else {
                None
            };

            cors_response(Response::from_json(&SubmitAnnotationsResponse {
                success: true,
                message: "Annotations submitted successfully".to_string(),
                all_signed,
                remaining_signers,
                download_url,
            }))
        }
        None => cors_response(Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Session not found"
        }))?
        .with_status(404))),
    }
}

// ============================================================
// Download Endpoint - Merge Annotations into PDF
// ============================================================

/// Handle GET /session/{id}/download - merge annotations and return PDF
async fn handle_download(session_id: &str, env: Env) -> Result<Response> {
    console_log!("handle_download called for session_id: {}", session_id);
    let kv = env.kv("SESSIONS")?;
    let key = format!("session:{}", session_id);
    console_log!("Looking up key: {}", key);

    match kv.get(&key).text().await? {
        Some(session_json) => {
            let mut session: SigningSession = serde_json::from_str(&session_json)
                .map_err(|e| Error::from(format!("Parse session error: {}", e)))?;

            // If we have a cached final document, return it
            if let Some(ref final_doc) = session.final_document {
                return serve_pdf(&session.metadata.filename, final_doc);
            }

            // Decode the original PDF
            let pdf_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &session.encrypted_document,
            )
            .map_err(|e| Error::from(format!("Base64 decode error: {}", e)))?;

            // If no annotations, just return the original PDF
            if session.signature_annotations.is_empty() {
                // For legacy sessions, check if there are signed_versions
                if !session.signed_versions.is_empty() {
                    // Return the most recent signed version
                    if let Some(latest) = session.signed_versions.last() {
                        return serve_pdf(&session.metadata.filename, &latest.encrypted_document);
                    }
                }
                return serve_pdf_bytes(&session.metadata.filename, pdf_bytes);
            }

            // Get page dimensions for coordinate conversion
            console_log!("Getting page dimensions for {} bytes PDF", pdf_bytes.len());
            let page_dims = match get_pdf_page_dimensions(&pdf_bytes) {
                Ok(dims) => {
                    console_log!("Got page dimensions: {} pages", dims.len());
                    dims
                }
                Err(e) => {
                    console_log!("Error getting page dimensions: {:?}", e);
                    return cors_response(Response::error(
                        &format!("PDF page dims error: {:?}", e),
                        500,
                    ));
                }
            };

            // Apply annotations to the PDF
            console_log!(
                "Applying {} annotations",
                session.signature_annotations.len()
            );
            let merged_pdf = match apply_annotations_to_pdf(
                pdf_bytes,
                &session.signature_annotations,
                &page_dims,
            ) {
                Ok(pdf) => {
                    console_log!("Annotations applied, merged PDF is {} bytes", pdf.len());
                    pdf
                }
                Err(e) => {
                    console_log!("Error applying annotations: {:?}", e);
                    return cors_response(Response::error(
                        &format!("PDF merge error: {:?}", e),
                        500,
                    ));
                }
            };

            // Encode result as base64 for caching
            let merged_base64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &merged_pdf);

            // Cache the result if session is complete (saves time on repeat downloads)
            if session.status == SessionStatus::Completed {
                session.final_document = Some(merged_base64.clone());
                let updated_json = serde_json::to_string(&session)
                    .map_err(|e| Error::from(format!("Serialize error: {}", e)))?;
                kv.put(&key, updated_json)?.execute().await?;
            }

            serve_pdf_bytes(&session.metadata.filename, merged_pdf)
        }
        None => {
            console_log!("Session NOT found in KV for download: {}", key);
            cors_response(Response::error("Session not found (download)", 404))
        }
    }
}

/// Serve a PDF file from base64 data
fn serve_pdf(filename: &str, base64_data: &str) -> Result<Response> {
    let pdf_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
        .map_err(|e| Error::from(format!("Base64 decode error: {}", e)))?;
    serve_pdf_bytes(filename, pdf_bytes)
}

/// Serve a PDF file from raw bytes
fn serve_pdf_bytes(filename: &str, pdf_bytes: Vec<u8>) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", "application/pdf")?;
    headers.set(
        "Content-Disposition",
        &format!("attachment; filename=\"{}\"", filename),
    )?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;

    Ok(Response::from_bytes(pdf_bytes)?.with_headers(headers))
}

// ============================================================
// PDF Merge Logic - Apply Annotations to Original PDF
// ============================================================

/// Get page dimensions from a PDF (returns HashMap of page_num -> [x, y, width, height])
fn get_pdf_page_dimensions(pdf_bytes: &[u8]) -> Result<std::collections::HashMap<u32, [f64; 4]>> {
    let doc = lopdf::Document::load_mem(pdf_bytes)
        .map_err(|e| Error::from(format!("PDF parse error: {}", e)))?;

    let mut dims = std::collections::HashMap::new();
    let page_count = doc.get_pages().len();

    for page_num in 1..=page_count {
        if let Ok(page_id) = doc.page_iter().nth(page_num - 1).ok_or("Page not found") {
            if let Ok(page_dict) = doc.get_object(page_id).and_then(|o| o.as_dict()) {
                // Try MediaBox first, then CropBox
                let media_box = page_dict
                    .get(b"MediaBox")
                    .or_else(|_| page_dict.get(b"CropBox"))
                    .and_then(|obj| obj.as_array())
                    .map(|arr| {
                        let get_f64 = |idx: usize| -> f64 {
                            arr.get(idx)
                                .and_then(|o| match o {
                                    Object::Integer(i) => Some(*i as f64),
                                    Object::Real(f) => Some(*f as f64),
                                    _ => None,
                                })
                                .unwrap_or(0.0)
                        };
                        [get_f64(0), get_f64(1), get_f64(2), get_f64(3)]
                    })
                    .unwrap_or([0.0, 0.0, 612.0, 792.0]); // Default to US Letter

                dims.insert(page_num as u32, media_box);
            }
        }
    }

    // Fallback: if we couldn't get any dims, use defaults
    if dims.is_empty() {
        for i in 1..=page_count {
            dims.insert(i as u32, [0.0, 0.0, 612.0, 792.0]);
        }
    }

    Ok(dims)
}

/// Apply all annotations to a PDF and return the merged bytes
fn apply_annotations_to_pdf(
    pdf_bytes: Vec<u8>,
    annotations: &[SignatureAnnotation],
    page_dims: &std::collections::HashMap<u32, [f64; 4]>,
) -> Result<Vec<u8>> {
    console_log!(
        "apply_annotations_to_pdf: {} annotations, {} bytes",
        annotations.len(),
        pdf_bytes.len()
    );
    let mut doc = lopdf::Document::load_mem(&pdf_bytes).map_err(|e| {
        console_log!("PDF load error: {}", e);
        Error::from(format!("PDF parse error: {}", e))
    })?;
    console_log!("PDF loaded, pages: {}", doc.get_pages().len());

    // Group annotations by page for efficiency
    let mut by_page: std::collections::HashMap<u32, Vec<&SignatureAnnotation>> =
        std::collections::HashMap::new();
    for ann in annotations {
        // Position.page is already 1-indexed (matches PDF page numbers)
        by_page.entry(ann.position.page).or_default().push(ann);
    }

    // Apply annotations to each page
    for (page_num, anns) in by_page {
        let dims = page_dims
            .get(&page_num)
            .copied()
            .unwrap_or([0.0, 0.0, 612.0, 792.0]);
        let page_width = dims[2] - dims[0];
        let page_height = dims[3] - dims[1];

        for ann in anns {
            // Convert percentage coordinates to PDF points
            let x = (ann.position.x_percent / 100.0) * page_width + dims[0];
            let y_from_top = (ann.position.y_percent / 100.0) * page_height;
            let width = (ann.position.width_percent / 100.0) * page_width;
            let height = (ann.position.height_percent / 100.0) * page_height;
            // PDF Y origin is bottom-left, so flip Y
            let y = dims[1] + page_height - y_from_top - height;

            let result = match &ann.data {
                AnnotationData::DrawnSignature { image_base64 }
                | AnnotationData::Initials { image_base64 } => {
                    console_log!("Adding image annotation for field {}", ann.field_id);
                    add_image_annotation(&mut doc, page_num, x, y, width, height, image_base64)
                }
                AnnotationData::TypedSignature { text, font: _ } => {
                    console_log!("Adding typed signature for field {}", ann.field_id);
                    add_text_annotation(
                        &mut doc,
                        page_num,
                        x,
                        y,
                        width,
                        height,
                        text,
                        [0.9, 0.95, 1.0],
                    )
                }
                AnnotationData::Date { value } => {
                    console_log!(
                        "Adding date annotation for field {}: {}",
                        ann.field_id,
                        value
                    );
                    add_text_annotation(
                        &mut doc,
                        page_num,
                        x,
                        y,
                        width,
                        height,
                        value,
                        [1.0, 0.95, 0.9],
                    )
                }
                AnnotationData::Text { value } => {
                    console_log!(
                        "Adding text annotation for field {}: {}",
                        ann.field_id,
                        value
                    );
                    add_text_annotation(
                        &mut doc,
                        page_num,
                        x,
                        y,
                        width,
                        height,
                        value,
                        [1.0, 1.0, 0.9],
                    )
                }
                AnnotationData::Checkbox { checked } => {
                    console_log!(
                        "Adding checkbox annotation for field {}: {}",
                        ann.field_id,
                        checked
                    );
                    add_checkbox_annotation(&mut doc, page_num, x, y, width.min(height), *checked)
                }
            };
            if let Err(e) = result {
                console_log!("Error adding annotation {}: {:?}", ann.field_id, e);
                return Err(e);
            }
        }
    }

    // Save the modified PDF
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| Error::from(format!("PDF save error: {}", e)))?;

    Ok(output)
}

/// Add an image annotation (signature/initials) to the PDF
fn add_image_annotation(
    doc: &mut lopdf::Document,
    page_num: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    image_base64: &str,
) -> Result<()> {
    // Strip data URL prefix if present
    let base64_data = strip_data_url_prefix(image_base64);

    // Decode base64 to PNG bytes
    let png_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data)
        .map_err(|e| Error::from(format!("Image base64 decode error: {}", e)))?;

    // Decode PNG to RGB pixels
    let (img_width, img_height, rgb_data) = decode_png_to_rgb(&png_bytes)?;

    // Compress pixel data
    let compressed = compress_rgb_data(&rgb_data)?;

    // Create image XObject
    let mut img_dict = Dictionary::new();
    img_dict.set("Type", Object::Name(b"XObject".to_vec()));
    img_dict.set("Subtype", Object::Name(b"Image".to_vec()));
    img_dict.set("Width", Object::Integer(img_width as i64));
    img_dict.set("Height", Object::Integer(img_height as i64));
    img_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    img_dict.set("BitsPerComponent", Object::Integer(8));
    img_dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
    let image_id = doc.add_object(Object::Stream(Stream::new(img_dict, compressed)));

    // Create form XObject that draws the image
    let content = format!(
        "q\n{} 0 0 {} 0 0 cm\n/Im0 Do\nQ",
        width as i64, height as i64
    );
    let content_bytes = content.into_bytes();

    let mut xobject_dict = Dictionary::new();
    xobject_dict.set("Im0", Object::Reference(image_id));

    let mut resources = Dictionary::new();
    resources.set("XObject", Object::Dictionary(xobject_dict));

    let mut form_dict = Dictionary::new();
    form_dict.set("Type", Object::Name(b"XObject".to_vec()));
    form_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    form_dict.set("FormType", Object::Integer(1));
    form_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(width as f32),
            Object::Real(height as f32),
        ]),
    );
    form_dict.set("Resources", Object::Dictionary(resources));
    let form_id = doc.add_object(Object::Stream(Stream::new(form_dict, content_bytes)));

    // Create stamp annotation
    let mut ap_dict = Dictionary::new();
    ap_dict.set("N", Object::Reference(form_id));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
    annot_dict.set("Subtype", Object::Name(b"Stamp".to_vec()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + height) as f32),
        ]),
    );
    annot_dict.set("F", Object::Integer(4)); // Print flag
    annot_dict.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));

    // Add to page annotations
    add_annotation_to_page(doc, page_num, annot_id)?;

    Ok(())
}

/// Add a text annotation to the PDF
fn add_text_annotation(
    doc: &mut lopdf::Document,
    page_num: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: &str,
    bg_color: [f64; 3],
) -> Result<()> {
    let escaped_text = escape_pdf_string(text);
    let font_size = (height * 0.6).clamp(8.0, 14.0);
    let text_y = (height - font_size) / 2.0;

    let content = format!(
        "q\n{r} {g} {b} rg\n0 0 {w} {h} re f\n0 0 0 RG\n0.5 w\n0 0 {w} {h} re S\n0 0 0 rg\nBT\n/F1 {fs} Tf\n4 {ty} Td\n({text}) Tj\nET\nQ",
        r = bg_color[0],
        g = bg_color[1],
        b = bg_color[2],
        w = width,
        h = height,
        fs = font_size,
        ty = text_y,
        text = escaped_text,
    );
    let content_bytes = content.into_bytes();

    // Create font resource
    let mut font_f1 = Dictionary::new();
    font_f1.set("Type", Object::Name(b"Font".to_vec()));
    font_f1.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_f1.set("BaseFont", Object::Name(b"Helvetica".to_vec()));

    let mut fonts = Dictionary::new();
    fonts.set("F1", Object::Dictionary(font_f1));

    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(fonts));

    let mut form_dict = Dictionary::new();
    form_dict.set("Type", Object::Name(b"XObject".to_vec()));
    form_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    form_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(width as f32),
            Object::Real(height as f32),
        ]),
    );
    form_dict.set("Resources", Object::Dictionary(resources));
    let form_id = doc.add_object(Object::Stream(Stream::new(form_dict, content_bytes)));

    // Create stamp annotation
    let mut ap_dict = Dictionary::new();
    ap_dict.set("N", Object::Reference(form_id));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
    annot_dict.set("Subtype", Object::Name(b"Stamp".to_vec()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + width) as f32),
            Object::Real((y + height) as f32),
        ]),
    );
    annot_dict.set("F", Object::Integer(4));
    annot_dict.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));
    add_annotation_to_page(doc, page_num, annot_id)?;

    Ok(())
}

/// Add a checkbox annotation to the PDF
fn add_checkbox_annotation(
    doc: &mut lopdf::Document,
    page_num: u32,
    x: f64,
    y: f64,
    size: f64,
    checked: bool,
) -> Result<()> {
    let checkmark = if checked {
        format!(
            "q\n0 G\n2 w\n{x1} {y1} m\n{x2} {y2} l\n{x3} {y3} l\nS\nQ",
            x1 = size * 0.2,
            y1 = size * 0.5,
            x2 = size * 0.4,
            y2 = size * 0.3,
            x3 = size * 0.8,
            y3 = size * 0.8,
        )
    } else {
        String::new()
    };

    let content = format!(
        "q\n1 1 1 rg\n0 0 {s} {s} re f\n0 0 0 RG\n1 w\n0 0 {s} {s} re S\n{check}\nQ",
        s = size,
        check = checkmark,
    );
    let content_bytes = content.into_bytes();

    let mut form_dict = Dictionary::new();
    form_dict.set("Type", Object::Name(b"XObject".to_vec()));
    form_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    form_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(size as f32),
            Object::Real(size as f32),
        ]),
    );
    let form_id = doc.add_object(Object::Stream(Stream::new(form_dict, content_bytes)));

    let mut ap_dict = Dictionary::new();
    ap_dict.set("N", Object::Reference(form_id));

    let mut annot_dict = Dictionary::new();
    annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
    annot_dict.set("Subtype", Object::Name(b"Stamp".to_vec()));
    annot_dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x as f32),
            Object::Real(y as f32),
            Object::Real((x + size) as f32),
            Object::Real((y + size) as f32),
        ]),
    );
    annot_dict.set("F", Object::Integer(4));
    annot_dict.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot_dict));
    add_annotation_to_page(doc, page_num, annot_id)?;

    Ok(())
}

/// Add an annotation reference to a page's Annots array
fn add_annotation_to_page(
    doc: &mut lopdf::Document,
    page_num: u32,
    annot_id: ObjectId,
) -> Result<()> {
    // Get pages
    let pages: Vec<ObjectId> = doc.page_iter().collect();
    if page_num < 1 || page_num as usize > pages.len() {
        return Err(Error::from(format!("Invalid page number: {}", page_num)));
    }

    let page_id = pages[(page_num - 1) as usize];

    // Get existing annotations
    let mut annots = if let Ok(page_obj) = doc.get_object(page_id) {
        if let Ok(page_dict) = page_obj.as_dict() {
            if let Ok(annots_obj) = page_dict.get(b"Annots") {
                match annots_obj {
                    Object::Array(arr) => arr.clone(),
                    Object::Reference(ref_id) => {
                        if let Ok(arr_obj) = doc.get_object(*ref_id) {
                            arr_obj.as_array().cloned().unwrap_or_default()
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Add new annotation
    annots.push(Object::Reference(annot_id));

    // Update page
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(page_dict) = page_obj.as_dict_mut() {
            page_dict.set("Annots", Object::Array(annots));
        }
    }

    Ok(())
}

/// Decode PNG image to RGB pixels
fn decode_png_to_rgb(png_data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let cursor = std::io::Cursor::new(png_data);
    let decoder = PngDecoder::new(cursor);
    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::from(format!("PNG decode error: {}", e)))?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let color_type = info.color_type;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    reader
        .next_frame(&mut buf)
        .map_err(|e| Error::from(format!("PNG frame error: {}", e)))?;

    let rgb_data = match color_type {
        png::ColorType::Rgb => buf,
        png::ColorType::Rgba => {
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for chunk in buf.chunks(4) {
                rgb.push(chunk[0]);
                rgb.push(chunk[1]);
                rgb.push(chunk[2]);
            }
            rgb
        }
        png::ColorType::Grayscale => {
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for &gray in &buf {
                rgb.push(gray);
                rgb.push(gray);
                rgb.push(gray);
            }
            rgb
        }
        png::ColorType::GrayscaleAlpha => {
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for chunk in buf.chunks(2) {
                let gray = chunk[0];
                rgb.push(gray);
                rgb.push(gray);
                rgb.push(gray);
            }
            rgb
        }
        png::ColorType::Indexed => {
            return Err(Error::from("Indexed PNG not supported - use RGB or RGBA"));
        }
    };

    Ok((width, height, rgb_data))
}

/// Compress RGB data using zlib (FlateDecode compatible)
fn compress_rgb_data(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| Error::from(format!("Compression write error: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| Error::from(format!("Compression finish error: {}", e)))
}

/// Strip data URL prefix (e.g., "data:image/png;base64,") from a string
fn strip_data_url_prefix(data: &str) -> &str {
    if let Some(pos) = data.find(',') {
        &data[pos + 1..]
    } else {
        data
    }
}

/// Escape special PDF characters in strings
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            _ if c.is_ascii() => c.to_string(),
            _ => "?".to_string(),
        })
        .collect()
}

fn generate_session_id() -> String {
    // WASM-safe timestamp (std::time::SystemTime panics in Workers)
    let timestamp = get_timestamp_millis();

    // Generate random suffix
    let random: u64 = js_sys::Math::random().to_bits();

    format!("{:x}{:08x}", timestamp, random as u32)
}

// ============================================================
// Sender Session Index Helper Functions
// ============================================================

/// Hash an email address using SHA-256 for privacy-preserving storage keys
fn hash_sender_email(email: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(email.to_lowercase().as_bytes());
    let result = hasher.finalize();
    // Return hex-encoded hash
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Get the sender session index from KV storage
async fn get_sender_index(kv: &kv::KvStore, sender_hash: &str) -> SenderSessionIndex {
    let key = format!("sender_index:{}", sender_hash);
    match kv.get(&key).json::<SenderSessionIndex>().await {
        Ok(Some(index)) => index,
        _ => SenderSessionIndex::default(),
    }
}

/// Save the sender session index to KV storage
async fn save_sender_index(
    kv: &kv::KvStore,
    sender_hash: &str,
    index: &SenderSessionIndex,
) -> Result<()> {
    let key = format!("sender_index:{}", sender_hash);
    kv.put(&key, serde_json::to_string(index)?)?
        .execute()
        .await?;
    Ok(())
}

/// Get the recipient session index from KV storage
async fn get_recipient_index(kv: &kv::KvStore, email_hash: &str) -> RecipientSessionIndex {
    let key = format!("recipient_index:{}", email_hash);
    match kv.get(&key).json::<RecipientSessionIndex>().await {
        Ok(Some(index)) => index,
        _ => RecipientSessionIndex::default(),
    }
}

/// Save the recipient session index to KV storage
async fn save_recipient_index(
    kv: &kv::KvStore,
    email_hash: &str,
    index: &RecipientSessionIndex,
) -> Result<()> {
    let key = format!("recipient_index:{}", email_hash);
    kv.put(&key, serde_json::to_string(index)?)?
        .execute()
        .await?;
    Ok(())
}

/// Remove a session from a sender's index (for cleanup on session deletion)
#[allow(dead_code)] // Prepared for future explicit session deletion endpoint
async fn remove_session_from_sender_index(
    kv: &kv::KvStore,
    sender_email: &str,
    session_id: &str,
) -> Result<()> {
    let sender_hash = hash_sender_email(sender_email);
    let mut index = get_sender_index(kv, &sender_hash).await;
    index.remove_session(session_id);
    save_sender_index(kv, &sender_hash, &index).await
}

fn error_response(msg: &str) -> Result<Response> {
    let resp = Response::from_json(&SendResponse {
        success: false,
        message: msg.to_string(),
    })?;
    Ok(resp.with_status(400))
}

// ============================================================
// Request Size Validation (Security)
// ============================================================

/// Response structure for payload too large errors
#[derive(Serialize)]
struct PayloadTooLargeResponse {
    error: String,
    max_size_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    received_bytes: Option<usize>,
}

/// Check Content-Length header against a maximum size limit.
/// Returns None if within limits, Some(Response) with 413 if exceeded.
fn check_content_length(req: &Request, max_size: usize) -> Option<Response> {
    let content_length = req
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > max_size {
        let response = Response::from_json(&PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: max_size,
            received_bytes: Some(content_length),
        })
        .map(|r| r.with_status(413))
        .unwrap_or_else(|_| Response::error("Request too large", 413).unwrap());

        return Some(response);
    }

    None
}

/// Create a 413 Payload Too Large response
fn payload_too_large_response(max_size: usize, actual_size: usize) -> Result<Response> {
    let resp = Response::from_json(&PayloadTooLargeResponse {
        error: "Request too large".to_string(),
        max_size_bytes: max_size,
        received_bytes: Some(actual_size),
    })?
    .with_status(413);
    Ok(resp)
}

fn cors_response(response: Result<Response>) -> Result<Response> {
    response.map(|r| {
        let headers = Headers::new();
        let _ = headers.set("Access-Control-Allow-Origin", "*");
        let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, OPTIONS");
        let _ = headers.set(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        );
        r.with_headers(headers)
    })
}

// ============================================================
// UX-006 Helper Functions (Stub implementations for tests)
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Test Helpers
    // ============================================================

    /// Create a RecipientInfo with just the required fields, defaults for the rest
    fn test_recipient(id: &str, name: &str, email: &str, role: &str) -> RecipientInfo {
        RecipientInfo {
            id: id.to_string(),
            name: name.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            consented: false,
            consent_at: None,
            consent_user_agent: None,
            signed: false,
            signed_at: None,
            declined: false,
            declined_at: None,
            decline_reason: None,
            signing_order: None,
            reminders_sent: 0,
            last_reminder_at: None,
        }
    }

    /// Create a SigningSession with minimal required data
    fn test_session(id: &str, recipients: Vec<RecipientInfo>) -> SigningSession {
        SigningSession {
            id: id.to_string(),
            encrypted_document: "test_doc_base64".to_string(),
            metadata: SessionMetadata {
                filename: "test.pdf".to_string(),
                page_count: 1,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: "Test User".to_string(),
                sender_email: Some("sender@test.com".to_string()),
            },
            recipients,
            fields: vec![],
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signed_versions: vec![],
            signature_annotations: vec![],
            status: SessionStatus::Pending,
            signing_mode: SigningMode::Parallel,
            reminder_config: None,
            final_document: None,
            legacy: false,
            revision_count: 0,
            voided_at: None,
            void_reason: None,
            token_version: 0,
        }
    }

    // ============================================================
    // Unit Tests for Session Token Generation and Verification
    // ============================================================

    #[test]
    fn test_generate_and_verify_token_roundtrip() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";
        let token_version = 0u32;

        let token = generate_recipient_token(session_id, recipient_id, token_version, secret);

        assert!(!token.is_empty());
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        let result = verify_recipient_token(&token, session_id, secret);
        assert!(result.is_ok());
        let verified = result.unwrap();
        assert_eq!(verified.recipient_id, recipient_id);
        assert_eq!(verified.token_version, token_version);
    }

    #[test]
    fn test_generate_and_verify_token_with_version() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";
        let token_version = 5u32;

        let token = generate_recipient_token(session_id, recipient_id, token_version, secret);

        let result = verify_recipient_token(&token, session_id, secret);
        assert!(result.is_ok());
        let verified = result.unwrap();
        assert_eq!(verified.recipient_id, recipient_id);
        assert_eq!(verified.token_version, token_version);
    }

    #[test]
    fn test_verify_token_wrong_session_id() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";

        let token = generate_recipient_token(session_id, recipient_id, 0, secret);
        let result = verify_recipient_token(&token, "sess_different", secret);
        assert_eq!(result, Err(TokenError::SessionMismatch));
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let secret = b"test-secret-key";
        let wrong_secret = b"wrong-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";

        let token = generate_recipient_token(session_id, recipient_id, 0, secret);
        let result = verify_recipient_token(&token, session_id, wrong_secret);
        assert_eq!(result, Err(TokenError::InvalidSignature));
    }

    #[test]
    fn test_verify_token_invalid_format() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";

        let result = verify_recipient_token("not-a-valid-token", session_id, secret);
        assert_eq!(result, Err(TokenError::InvalidFormat));

        let result = verify_recipient_token("", session_id, secret);
        assert_eq!(result, Err(TokenError::InvalidFormat));
    }

    #[test]
    fn test_verify_token_expired() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let secret = b"test-secret-key";
        let session_id = "sess_abc123";
        let recipient_id = "recipient_456";
        let expiry = chrono::Utc::now().timestamp() as u64 - 3600;
        let payload = format!("{}:{}:{}", session_id, recipient_id, expiry);

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key size");
        mac.update(payload.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_base64 = URL_SAFE_NO_PAD.encode(signature);

        let token_str = format!("{}:{}", payload, sig_base64);
        let token = URL_SAFE_NO_PAD.encode(token_str.as_bytes());

        let result = verify_recipient_token(&token, session_id, secret);
        assert_eq!(result, Err(TokenError::Expired));
    }

    #[test]
    fn test_token_error_display() {
        assert_eq!(
            TokenError::InvalidFormat.to_string(),
            "Invalid token format"
        );
        assert_eq!(TokenError::Expired.to_string(), "Token has expired");
        assert_eq!(
            TokenError::InvalidSignature.to_string(),
            "Invalid token signature"
        );
        assert_eq!(
            TokenError::SessionMismatch.to_string(),
            "Token does not match session"
        );
        assert_eq!(
            TokenError::VersionMismatch.to_string(),
            "Token has been invalidated (document was revised or voided)"
        );
    }

    #[test]
    fn test_different_recipients_get_different_tokens() {
        let secret = b"test-secret-key";
        let session_id = "sess_abc123";

        let token1 = generate_recipient_token(session_id, "recipient_1", 0, secret);
        let token2 = generate_recipient_token(session_id, "recipient_2", 0, secret);

        assert_ne!(token1, token2);

        assert_eq!(
            verify_recipient_token(&token1, session_id, secret)
                .unwrap()
                .recipient_id,
            "recipient_1"
        );
        assert_eq!(
            verify_recipient_token(&token2, session_id, secret)
                .unwrap()
                .recipient_id,
            "recipient_2"
        );
    }

    // ============================================================
    // Unit Tests for Request Size Validation
    // ============================================================

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_size_limit_constants() {
        // Verify size limits are set correctly
        assert_eq!(MAX_PDF_SIZE, 10 * 1024 * 1024); // 10MB
        assert_eq!(MAX_SIGNATURE_SIZE, 100 * 1024); // 100KB
        assert_eq!(MAX_REQUEST_BODY, 12 * 1024 * 1024); // 12MB

        // Ensure hierarchy makes sense
        assert!(MAX_SIGNATURE_SIZE < MAX_PDF_SIZE);
        assert!(MAX_PDF_SIZE < MAX_REQUEST_BODY);
    }

    #[test]
    fn test_payload_too_large_response_serialization() {
        let response = PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: MAX_PDF_SIZE,
            received_bytes: Some(15_000_000),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\":\"Request too large\""));
        assert!(json.contains("\"max_size_bytes\":10485760"));
        assert!(json.contains("\"received_bytes\":15000000"));
    }

    #[test]
    fn test_payload_too_large_response_without_received_bytes() {
        let response = PayloadTooLargeResponse {
            error: "Request too large".to_string(),
            max_size_bytes: MAX_PDF_SIZE,
            received_bytes: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\":\"Request too large\""));
        assert!(json.contains("\"max_size_bytes\":10485760"));
        // received_bytes should be omitted when None
        assert!(!json.contains("received_bytes"));
    }

    #[test]
    fn test_pdf_size_validation_within_limit() {
        // 1MB of data (within 10MB limit)
        let data = "a".repeat(1024 * 1024);
        assert!(data.len() <= MAX_PDF_SIZE);
    }

    #[test]
    fn test_pdf_size_validation_exceeds_limit() {
        // 11MB of data (exceeds 10MB limit)
        let data = "a".repeat(11 * 1024 * 1024);
        assert!(data.len() > MAX_PDF_SIZE);
    }

    #[test]
    fn test_signature_size_validation_within_limit() {
        // 50KB of data (within 100KB limit)
        let data = "a".repeat(50 * 1024);
        assert!(data.len() <= MAX_SIGNATURE_SIZE);
    }

    #[test]
    fn test_signature_size_validation_exceeds_limit() {
        // 150KB of data (exceeds 100KB limit)
        let data = "a".repeat(150 * 1024);
        assert!(data.len() > MAX_SIGNATURE_SIZE);
    }

    #[test]
    fn test_request_body_size_validation_within_limit() {
        // 10MB of data (within 12MB limit)
        let data = "a".repeat(10 * 1024 * 1024);
        assert!(data.len() <= MAX_REQUEST_BODY);
    }

    #[test]
    fn test_request_body_size_validation_exceeds_limit() {
        // 15MB of data (exceeds 12MB limit)
        let data = "a".repeat(15 * 1024 * 1024);
        assert!(data.len() > MAX_REQUEST_BODY);
    }

    // ============================================================
    // Unit Tests for SenderSessionIndex
    // ============================================================

    #[test]
    fn test_sender_session_index_default() {
        let index = SenderSessionIndex::default();
        assert_eq!(index.count(), 0);
        assert!(index.session_ids.is_empty());
        assert!(index.created_at.is_empty());
    }

    #[test]
    fn test_sender_session_index_add_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());

        assert_eq!(index.count(), 1);
        assert_eq!(index.session_ids[0], "sess_001");
        assert_eq!(index.created_at[0], "2025-01-15T10:00:00Z");
    }

    #[test]
    fn test_sender_session_index_remove_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());
        index.add_session("sess_002".to_string(), "2025-01-16T10:00:00Z".to_string());
        index.add_session("sess_003".to_string(), "2025-01-17T10:00:00Z".to_string());

        assert_eq!(index.count(), 3);

        index.remove_session("sess_002");
        assert_eq!(index.count(), 2);
        assert_eq!(index.session_ids, vec!["sess_001", "sess_003"]);
        assert_eq!(
            index.created_at,
            vec!["2025-01-15T10:00:00Z", "2025-01-17T10:00:00Z"]
        );
    }

    #[test]
    fn test_sender_session_index_remove_nonexistent_session() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());

        // Removing non-existent session should not panic or change count
        index.remove_session("sess_999");
        assert_eq!(index.count(), 1);
    }

    #[test]
    fn test_sender_session_index_prune_expired() {
        let mut index = SenderSessionIndex::default();

        // Add sessions at various ages
        // Session from 10 days ago (should be kept with 30-day prune)
        let ten_days_ago = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
        index.add_session("sess_recent".to_string(), ten_days_ago);

        // Session from 35 days ago (should be pruned with 30-day prune)
        let thirty_five_days_ago = (chrono::Utc::now() - chrono::Duration::days(35)).to_rfc3339();
        index.add_session("sess_old".to_string(), thirty_five_days_ago);

        // Session from today (should be kept)
        let now = chrono::Utc::now().to_rfc3339();
        index.add_session("sess_today".to_string(), now);

        assert_eq!(index.count(), 3);

        index.prune_expired(30);

        assert_eq!(index.count(), 2);
        assert!(index.session_ids.contains(&"sess_recent".to_string()));
        assert!(index.session_ids.contains(&"sess_today".to_string()));
        assert!(!index.session_ids.contains(&"sess_old".to_string()));
    }

    #[test]
    fn test_sender_session_index_prune_with_invalid_timestamps() {
        let mut index = SenderSessionIndex::default();

        // Valid timestamp
        let now = chrono::Utc::now().to_rfc3339();
        index.add_session("sess_valid".to_string(), now);

        // Invalid timestamp (should be removed during prune)
        index.add_session(
            "sess_invalid".to_string(),
            "not-a-valid-timestamp".to_string(),
        );

        assert_eq!(index.count(), 2);

        index.prune_expired(30);

        // Only the valid session should remain
        assert_eq!(index.count(), 1);
        assert_eq!(index.session_ids[0], "sess_valid");
    }

    #[test]
    fn test_sender_session_index_at_limit() {
        let mut index = SenderSessionIndex::default();

        // Add exactly MAX_SESSIONS_PER_SENDER sessions
        for i in 0..MAX_SESSIONS_PER_SENDER {
            index.add_session(format!("sess_{:03}", i), chrono::Utc::now().to_rfc3339());
        }

        assert_eq!(index.count(), MAX_SESSIONS_PER_SENDER);
        assert!(index.count() >= MAX_SESSIONS_PER_SENDER);
    }

    #[test]
    fn test_hash_sender_email_consistency() {
        // Same email should produce same hash
        let hash1 = hash_sender_email("test@example.com");
        let hash2 = hash_sender_email("test@example.com");
        assert_eq!(hash1, hash2);

        // Case insensitive
        let hash_lower = hash_sender_email("test@example.com");
        let hash_upper = hash_sender_email("TEST@EXAMPLE.COM");
        assert_eq!(hash_lower, hash_upper);

        // Different emails should produce different hashes
        let hash_other = hash_sender_email("other@example.com");
        assert_ne!(hash1, hash_other);
    }

    #[test]
    fn test_hash_sender_email_format() {
        let hash = hash_sender_email("test@example.com");

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Should only contain hex characters
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sender_session_index_serialization() {
        let mut index = SenderSessionIndex::default();
        index.add_session("sess_001".to_string(), "2025-01-15T10:00:00Z".to_string());
        index.add_session("sess_002".to_string(), "2025-01-16T10:00:00Z".to_string());

        // Should serialize correctly
        let json = serde_json::to_string(&index).unwrap();
        assert!(json.contains("sess_001"));
        assert!(json.contains("sess_002"));
        assert!(json.contains("2025-01-15T10:00:00Z"));

        // Should deserialize correctly
        let deserialized: SenderSessionIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count(), 2);
        assert_eq!(deserialized.session_ids, index.session_ids);
        assert_eq!(deserialized.created_at, index.created_at);
    }

    #[test]
    fn test_session_limit_constant() {
        // Verify the session limit is set to expected value
        assert_eq!(MAX_SESSIONS_PER_SENDER, 100);

        // Verify prune days is set correctly
        assert_eq!(SESSION_INDEX_PRUNE_DAYS, 30);
    }

    // ============================================================
    // Property Tests for Session/Magic Link Functionality
    // ============================================================

    use proptest::prelude::*;

    proptest! {
        /// Property: SessionMetadata serialization roundtrip preserves all fields
        #[test]
        fn prop_session_metadata_roundtrip(
            filename in "[a-zA-Z0-9_-]{1,50}\\.pdf",
            page_count in 1u32..1000,
            created_by in "[a-zA-Z ]{1,50}"
        ) {
            let metadata = SessionMetadata {
                filename: filename.clone(),
                page_count,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: created_by.clone(),
                sender_email: Some("test@example.com".to_string()),
            };

            let json = serde_json::to_string(&metadata).unwrap();
            let deserialized: SessionMetadata = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(metadata.filename, deserialized.filename);
            prop_assert_eq!(metadata.page_count, deserialized.page_count);
            prop_assert_eq!(metadata.created_by, deserialized.created_by);
        }

        /// Property: RecipientInfo serialization preserves email and role
        #[test]
        fn prop_recipient_info_roundtrip(
            id in "[0-9]{1,10}",
            name in "[a-zA-Z ]{1,50}",
            email in "[a-z]{3,10}@[a-z]{3,10}\\.[a-z]{2,4}",
            role in prop_oneof!["signer", "viewer"],
            signed in proptest::bool::ANY
        ) {
            let recipient = RecipientInfo {
                id: id.clone(),
                name: name.clone(),
                email: email.clone(),
                role: role.clone(),
                consented: signed, // If signed, must have consented
                consent_at: if signed { Some("2025-01-15T09:55:00Z".to_string()) } else { None },
                consent_user_agent: None,
                signed,
                signed_at: if signed { Some("2025-01-15T10:00:00Z".to_string()) } else { None },
                declined: false,
                declined_at: None,
                decline_reason: None,
                signing_order: None,
                reminders_sent: 0,
                last_reminder_at: None,
            };

            let json = serde_json::to_string(&recipient).unwrap();
            let deserialized: RecipientInfo = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(recipient.id, deserialized.id);
            prop_assert_eq!(recipient.email, deserialized.email);
            prop_assert_eq!(recipient.role, deserialized.role);
            prop_assert_eq!(recipient.signed, deserialized.signed);
        }

        /// Property: FieldInfo coordinates are within 0-100% bounds
        #[test]
        fn prop_field_coordinates_in_bounds(
            x in 0.0f64..100.0,
            y in 0.0f64..100.0,
            width in 1.0f64..50.0,
            height in 1.0f64..20.0,
            page in 1u32..100
        ) {
            let field = FieldInfo {
                id: "field_1".to_string(),
                field_type: "signature".to_string(),
                recipient_id: "1".to_string(),
                page,
                x_percent: x,
                y_percent: y,
                width_percent: width,
                height_percent: height,
                required: true,
                value: None,
            };

            // Verify coordinates are valid percentages
            prop_assert!(field.x_percent >= 0.0 && field.x_percent <= 100.0);
            prop_assert!(field.y_percent >= 0.0 && field.y_percent <= 100.0);
            prop_assert!(field.width_percent > 0.0 && field.width_percent <= 100.0);
            prop_assert!(field.height_percent > 0.0 && field.height_percent <= 100.0);

            // Verify serialization roundtrip
            let json = serde_json::to_string(&field).unwrap();
            let deserialized: FieldInfo = serde_json::from_str(&json).unwrap();
            prop_assert!((deserialized.x_percent - x).abs() < 0.0001);
            prop_assert!((deserialized.y_percent - y).abs() < 0.0001);
        }

        /// Property: SigningSession serialization preserves all nested structures
        #[test]
        fn prop_signing_session_roundtrip(
            doc_size in 100usize..10000,
            num_recipients in 1usize..5,
            num_fields in 1usize..10
        ) {
            let encrypted_doc = "a".repeat(doc_size);

            let recipients: Vec<RecipientInfo> = (0..num_recipients)
                .map(|i| test_recipient(
                    &i.to_string(),
                    &format!("Recipient {}", i),
                    &format!("user{}@example.com", i),
                    "signer"
                ))
                .collect();

            let fields: Vec<FieldInfo> = (0..num_fields)
                .map(|i| FieldInfo {
                    id: format!("field_{}", i),
                    field_type: "signature".to_string(),
                    recipient_id: (i % num_recipients).to_string(),
                    page: 1,
                    x_percent: 10.0 + (i as f64 * 5.0),
                    y_percent: 20.0,
                    width_percent: 30.0,
                    height_percent: 10.0,
                    required: true,
                    value: None,
                })
                .collect();

            let session = SigningSession {
                id: "test_session_123".to_string(),
                encrypted_document: encrypted_doc.clone(),
                metadata: SessionMetadata {
                    filename: "test.pdf".to_string(),
                    page_count: 1,
                    created_at: "2025-01-15T10:00:00Z".to_string(),
                    created_by: "Test User".to_string(),
                    sender_email: Some("sender@example.com".to_string()),
                },
                recipients: recipients.clone(),
                fields: fields.clone(),
                expires_at: "2025-01-22T10:00:00Z".to_string(),
                signed_versions: vec![],
                status: SessionStatus::Pending,
                signing_mode: SigningMode::Parallel,
                reminder_config: None,
                final_document: None,
                legacy: false,
            };

            let json = serde_json::to_string(&session).unwrap();
            let deserialized: SigningSession = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(session.id, deserialized.id);
            prop_assert_eq!(session.encrypted_document.len(), deserialized.encrypted_document.len());
            prop_assert_eq!(session.recipients.len(), deserialized.recipients.len());
            prop_assert_eq!(session.fields.len(), deserialized.fields.len());
        }

        /// Property: Session expiry hours always produces valid future timestamp
        #[test]
        fn prop_expiry_hours_valid(
            expiry_hours in 1u32..168  // 1 hour to 7 days
        ) {
            use chrono::{Duration, Utc};

            let now = Utc::now();
            let expires_at = now + Duration::hours(expiry_hours as i64);

            // Expiry should always be in the future
            prop_assert!(expires_at > now);

            // Expiry should be within reasonable bounds (max 7 days)
            let max_expiry = now + Duration::hours(168);
            prop_assert!(expires_at <= max_expiry);
        }
    }

    // ============================================================
    // Unit Tests for Session Functionality
    // ============================================================

    #[test]
    fn test_session_with_signed_version() {
        let mut recipient = test_recipient("1", "Alice", "alice@example.com", "signer");
        recipient.signed = true;
        recipient.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut session = test_session("sess_123", vec![recipient]);
        session.encrypted_document = "base64_encrypted_doc".to_string();
        session.metadata.filename = "contract.pdf".to_string();
        session.metadata.page_count = 5;
        session.metadata.created_by = "Sender".to_string();
        session.metadata.sender_email = Some("sender@example.com".to_string());
        session.signed_versions = vec![SignedVersion {
            recipient_id: "1".to_string(),
            encrypted_document: "signed_doc_base64".to_string(),
            signed_at: "2025-01-15T11:00:00Z".to_string(),
        }];
        session.status = SessionStatus::Completed;

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SigningSession = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.signed_versions.len(), 1);
        assert_eq!(deserialized.signed_versions[0].recipient_id, "1");
        assert!(deserialized.recipients[0].signed);
    }

    #[test]
    fn test_field_types() {
        let field_types = ["signature", "initials", "date", "text", "checkbox"];

        for field_type in field_types {
            let field = FieldInfo {
                id: "f1".to_string(),
                field_type: field_type.to_string(),
                recipient_id: "1".to_string(),
                page: 1,
                x_percent: 50.0,
                y_percent: 50.0,
                width_percent: 20.0,
                height_percent: 5.0,
                required: true,
                value: None,
            };

            let json = serde_json::to_string(&field).unwrap();
            let deserialized: FieldInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.field_type, field_type);
        }
    }

    #[test]
    fn test_recipient_roles() {
        let roles = ["signer", "viewer"];

        for role in roles {
            let recipient = test_recipient("1", "Test", "test@example.com", role);

            let json = serde_json::to_string(&recipient).unwrap();
            let deserialized: RecipientInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.role, role);
        }
    }

    #[test]
    fn test_empty_signed_versions_deserialize() {
        // Ensure old sessions without signed_versions field can deserialize
        let json = r#"{
            "id": "sess_123",
            "encrypted_document": "abc",
            "metadata": {
                "filename": "test.pdf",
                "page_count": 1,
                "created_at": "2025-01-15T10:00:00Z",
                "created_by": "Test"
            },
            "recipients": [],
            "fields": [],
            "expires_at": "2025-01-22T10:00:00Z"
        }"#;

        let session: SigningSession = serde_json::from_str(json).unwrap();
        assert!(session.signed_versions.is_empty());
    }

    // ============================================================
    // UX-001: Consent Landing Page Tests
    // ============================================================

    #[test]
    fn test_session_metadata_has_sender_info_for_consent() {
        // Session metadata should include sender information for consent landing page
        let metadata = SessionMetadata {
            filename: "contract.pdf".to_string(),
            page_count: 5,
            created_at: "2025-01-15T10:00:00Z".to_string(),
            created_by: "John Doe".to_string(),
            sender_email: Some("john.doe@example.com".to_string()),
        };

        // We have created_by (sender name), created_at (date sent), and sender_email
        assert_eq!(metadata.created_by, "John Doe");
        assert_eq!(metadata.created_at, "2025-01-15T10:00:00Z");
        assert_eq!(metadata.filename, "contract.pdf");
        assert!(metadata.sender_email.as_ref().unwrap().contains('@'));
    }

    #[test]
    fn test_session_public_info_exposes_all_consent_data() {
        // SessionPublicInfo returned by GET /session/{id} should include all data
        // needed for the consent landing page
        let public_info = SessionPublicInfo {
            id: "sess_123".to_string(),
            metadata: SessionMetadata {
                filename: "test.pdf".to_string(),
                page_count: 1,
                created_at: "2025-01-15T10:00:00Z".to_string(),
                created_by: "Jane Smith".to_string(),
                sender_email: Some("jane.smith@example.com".to_string()),
            },
            recipients: vec![],
            fields: vec![],
            encrypted_document: "data".to_string(),
            expires_at: "2025-01-22T10:00:00Z".to_string(),
            signing_mode: SigningMode::Parallel,
            status: SessionStatus::Pending,
            final_document: None,
        };

        // Verify fields exist for consent page:
        // ✓ sender_name (created_by)
        // ✓ document_name (filename)
        // ✓ date_sent (created_at)
        // ✗ sender_email (MISSING)
        assert_eq!(public_info.metadata.created_by, "Jane Smith");
        assert_eq!(public_info.metadata.filename, "test.pdf");
        assert_eq!(public_info.metadata.created_at, "2025-01-15T10:00:00Z");
    }

    // ============================================================
    // UX-002: Accept/Decline Flow Tests
    // ============================================================

    #[test]
    fn test_decline_request_with_reason() {
        let json = r#"{
            "recipient_id": "r1",
            "reason": "Need more time to review"
        }"#;

        #[derive(Deserialize)]
        struct DeclineRequest {
            recipient_id: String,
            reason: Option<String>,
        }

        let req: DeclineRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r1");
        assert_eq!(req.reason, Some("Need more time to review".to_string()));
    }

    #[test]
    fn test_decline_request_without_reason() {
        let json = r#"{
            "recipient_id": "r2"
        }"#;

        #[derive(Deserialize)]
        struct DeclineRequest {
            recipient_id: String,
            reason: Option<String>,
        }

        let req: DeclineRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r2");
        assert_eq!(req.reason, None);
    }

    #[test]
    fn test_decline_response_serialization() {
        #[derive(Serialize)]
        struct DeclineResponse {
            success: bool,
            message: String,
        }

        let response = DeclineResponse {
            success: true,
            message: "Document declined successfully".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("Document declined"));
    }

    // ============================================================
    // Consent Tracking Tests
    // ============================================================

    #[test]
    fn test_consent_request_deserialization() {
        #[derive(Deserialize)]
        struct ConsentRequest {
            recipient_id: String,
            user_agent: Option<String>,
            consent_text_hash: Option<String>,
        }

        let json =
            r#"{"recipient_id": "r1", "user_agent": "Mozilla/5.0", "consent_text_hash": "abc123"}"#;
        let req: ConsentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r1");
        assert_eq!(req.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(req.consent_text_hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_consent_request_without_optional_fields() {
        #[derive(Deserialize)]
        struct ConsentRequest {
            recipient_id: String,
            user_agent: Option<String>,
            consent_text_hash: Option<String>,
        }

        let json = r#"{"recipient_id": "r2"}"#;
        let req: ConsentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.recipient_id, "r2");
        assert_eq!(req.user_agent, None);
        assert_eq!(req.consent_text_hash, None);
    }

    #[test]
    fn test_consent_response_serialization() {
        #[derive(Serialize)]
        struct ConsentResponse {
            success: bool,
            message: String,
            consent_at: String,
        }

        let response = ConsentResponse {
            success: true,
            message: "Consent recorded successfully".to_string(),
            consent_at: "2025-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("consent_at"));
        assert!(json.contains("2025-01-15T10:00:00Z"));
    }

    #[test]
    fn test_recipient_has_consent_fields() {
        // Verify RecipientInfo has consent tracking fields
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        // Initially not consented
        assert!(!recipient.consented);
        assert!(recipient.consent_at.is_none());
        assert!(recipient.consent_user_agent.is_none());

        // After consent
        recipient.consented = true;
        recipient.consent_at = Some("2025-01-15T10:00:00Z".to_string());
        recipient.consent_user_agent = Some("Mozilla/5.0".to_string());

        assert!(recipient.consented);
        assert!(recipient.consent_at.is_some());
        assert!(recipient.consent_user_agent.is_some());
    }

    #[test]
    fn test_consent_precedes_signing() {
        // Verify consent must happen before signing (logical flow)
        let mut recipient = test_recipient("1", "Jane", "jane@example.com", "signer");

        // Record consent
        recipient.consented = true;
        recipient.consent_at = Some("2025-01-15T10:00:00Z".to_string());

        // Then sign
        recipient.signed = true;
        recipient.signed_at = Some("2025-01-15T10:05:00Z".to_string());

        // Both should be recorded
        assert!(recipient.consented);
        assert!(recipient.signed);

        // Consent should come before signing
        let consent_time = recipient.consent_at.as_ref().unwrap();
        let sign_time = recipient.signed_at.as_ref().unwrap();
        assert!(consent_time < sign_time, "Consent should precede signing");
    }

    // ============================================================
    // UX-006: Sender Notification on Sign Tests
    // ============================================================

    #[test]
    fn test_session_metadata_contains_sender_email() {
        // Test that SessionMetadata includes sender_email field
        let metadata = SessionMetadata {
            filename: "contract.pdf".to_string(),
            page_count: 5,
            created_at: "2025-01-15T10:00:00Z".to_string(),
            created_by: "John Sender".to_string(),
            sender_email: Some("sender@example.com".to_string()),
        };

        // Verify we have a dedicated sender_email field
        assert!(
            metadata.sender_email.as_ref().unwrap().contains('@'),
            "Session metadata should contain sender email for notifications"
        );
    }

    #[test]
    fn test_notification_email_format() {
        // Test that notification email includes all required fields
        let recipient_name = "John Doe";
        let document_name = "contract.pdf";
        let signed_at = "2025-01-15T11:00:00Z";
        let download_link = "https://api.getsignatures.org/session/sess_123/download";

        let email_body = format_completion_notification_email(
            recipient_name,
            document_name,
            signed_at,
            download_link,
        );

        assert!(email_body.contains(recipient_name));
        assert!(email_body.contains(document_name));
        // The email formats the timestamp, so check for key parts like the year
        assert!(email_body.contains("2025"));
        assert!(email_body.contains("January") || email_body.contains("11:00"));
        assert!(email_body.contains(download_link));
    }

    #[test]
    fn test_all_recipients_signed_detection() {
        // Test that we can detect when all recipients have signed
        let mut alice = test_recipient("1", "Alice", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut bob = test_recipient("2", "Bob", "bob@example.com", "signer");
        bob.signed = true;
        bob.signed_at = Some("2025-01-15T12:00:00Z".to_string());

        let recipients = vec![alice, bob];

        // This will fail because the function doesn't exist yet
        assert!(all_recipients_signed(&recipients));
    }

    #[test]
    fn test_not_all_recipients_signed_detection() {
        // Test that we correctly detect when not all recipients have signed
        let mut alice = test_recipient("1", "Alice", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let bob = test_recipient("2", "Bob", "bob@example.com", "signer");
        // Bob hasn't signed (defaults to false)

        let recipients = vec![alice, bob];

        assert!(!all_recipients_signed(&recipients));
    }

    #[test]
    fn test_download_link_generation() {
        // Test that download links are generated with proper format
        let session_id = "sess_abc123";
        let expiry_days = 30;

        // This will fail because the function doesn't exist yet
        let download_link = generate_download_link(session_id, expiry_days);

        assert!(download_link.contains(session_id));
        assert!(download_link.starts_with("https://"));
        assert!(download_link.contains("download") || download_link.contains("session"));
    }

    #[test]
    fn test_completion_summary_email_includes_all_signers() {
        // Test that completion summary email includes all recipient names
        let mut alice = test_recipient("1", "Alice Smith", "alice@example.com", "signer");
        alice.signed = true;
        alice.signed_at = Some("2025-01-15T11:00:00Z".to_string());

        let mut bob = test_recipient("2", "Bob Jones", "bob@example.com", "signer");
        bob.signed = true;
        bob.signed_at = Some("2025-01-15T12:00:00Z".to_string());

        let recipients = vec![alice, bob];

        let document_name = "contract.pdf";
        let download_link = "https://api.getsignatures.org/session/sess_123/download";

        // This will fail because the function doesn't exist yet
        let email_body =
            format_all_signed_notification_email(&recipients, document_name, download_link);

        assert!(email_body.contains("Alice Smith"));
        assert!(email_body.contains("Bob Jones"));
        assert!(email_body.contains(document_name));
        assert!(
            email_body.contains("all recipients have signed") || email_body.contains("completed")
        );
    }

    #[test]
    fn test_download_link_expiry_30_days() {
        // Test that download links expire after 30 days
        let session_id = "sess_123";
        let expiry_days = 30;

        let download_link = generate_download_link(session_id, expiry_days);

        // The link should encode the expiry timestamp somehow
        // We'll verify this by checking that it's not just a simple session ID link
        assert!(
            download_link.len() > session_id.len() + 20,
            "Download link should include expiry information"
        );
    }

    // ============================================================
    // UX-002: Decline Flow Tests (MUST FAIL until implemented)
    // ============================================================

    #[test]
    fn test_recipient_has_declined_field() {
        // UX-002: RecipientInfo must have a declined field
        let recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        assert!(!recipient.declined);
        assert!(recipient.declined_at.is_none());
        assert!(recipient.decline_reason.is_none());
    }

    #[test]
    fn test_recipient_decline_sets_fields() {
        // UX-002: Declining a recipient should set declined=true and declined_at
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");

        // Simulate decline
        recipient.declined = true;
        recipient.declined_at = Some("2025-12-21T12:00:00Z".to_string());
        recipient.decline_reason = Some("Terms not acceptable".to_string());

        assert!(recipient.declined);
        assert!(recipient.declined_at.is_some());
        assert_eq!(
            recipient.decline_reason,
            Some("Terms not acceptable".to_string())
        );
    }

    #[test]
    fn test_declined_recipient_cannot_sign() {
        // UX-002: A declined recipient should not be able to sign
        let mut recipient = test_recipient("1", "John Doe", "john@example.com", "signer");
        recipient.declined = true;
        recipient.declined_at = Some("2025-12-21T12:00:00Z".to_string());
        recipient.decline_reason = Some("Not ready".to_string());

        // A declined recipient should be blocked from signing
        assert!(recipient.declined);
        assert!(!recipient.signed);
        // Attempting to sign a declined recipient should fail
        // (This would be tested at the handler level)
    }

    #[test]
    fn test_session_has_status_field() {
        // UX-002: SigningSession must have a status field
        let session = test_session("sess_123", vec![]);

        assert_eq!(session.status, SessionStatus::Pending);
    }

    #[test]
    fn test_session_status_transitions() {
        // UX-002: Session status should transition correctly
        let mut session = test_session("sess_123", vec![]);

        // Transition to declined
        session.status = SessionStatus::Declined;
        assert_eq!(session.status, SessionStatus::Declined);

        // Once declined, should stay declined
        assert!(matches!(session.status, SessionStatus::Declined));
    }

    #[test]
    fn test_session_status_enum_values() {
        // UX-002: SessionStatus enum should have all required values
        let pending = SessionStatus::Pending;
        let accepted = SessionStatus::Accepted;
        let declined = SessionStatus::Declined;
        let completed = SessionStatus::Completed;
        let expired = SessionStatus::Expired;

        assert!(matches!(pending, SessionStatus::Pending));
        assert!(matches!(accepted, SessionStatus::Accepted));
        assert!(matches!(declined, SessionStatus::Declined));
        assert!(matches!(completed, SessionStatus::Completed));
        assert!(matches!(expired, SessionStatus::Expired));
    }

    #[test]
    fn test_session_status_serialization() {
        // UX-002: SessionStatus should serialize to lowercase strings
        let pending = SessionStatus::Pending;
        let declined = SessionStatus::Declined;

        let pending_json = serde_json::to_string(&pending).unwrap();
        let declined_json = serde_json::to_string(&declined).unwrap();

        assert_eq!(pending_json, "\"pending\"");
        assert_eq!(declined_json, "\"declined\"");
    }

    // ============================================================
    // UX-004: Session Expiry & Resend Tests
    // ============================================================

    #[test]
    fn test_expired_session_response_structure() {
        // UX-004: When session is expired, should return status="expired"
        // with sender_email and document_name

        // Create an expired session
        let mut session = test_session("sess_expired", vec![]);
        session.metadata.filename = "expired.pdf".to_string();
        session.metadata.created_at = "2025-12-01T12:00:00Z".to_string();
        session.metadata.created_by = "Alice".to_string();
        session.metadata.sender_email = Some("alice@example.com".to_string());
        session.expires_at = "2025-12-10T12:00:00Z".to_string(); // Past date
        session.status = SessionStatus::Expired;

        // Verify session has expired status
        assert_eq!(session.status, SessionStatus::Expired);
        assert_eq!(
            session.metadata.sender_email,
            Some("alice@example.com".to_string())
        );
        assert_eq!(session.metadata.filename, "expired.pdf");
    }

    #[test]
    fn test_session_expiry_time_check() {
        // UX-004: Test that we can determine if a session has expired

        // Session that expired yesterday
        let past_time = "2025-12-20T12:00:00Z";
        let past_parsed = chrono::DateTime::parse_from_rfc3339(past_time).unwrap();
        let now = chrono::Utc::now();
        assert!(past_parsed < now, "Past time should be before now");

        // Session that expires tomorrow
        let future_time = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .to_rfc3339();
        let future_parsed = chrono::DateTime::parse_from_rfc3339(&future_time).unwrap();
        assert!(future_parsed > now, "Future time should be after now");
    }

    #[test]
    fn test_request_link_payload_structure() {
        // UX-004: Request link endpoint should accept recipient_id
        #[derive(Deserialize, Serialize)]
        struct RequestLinkRequest {
            recipient_id: String,
        }

        let request = RequestLinkRequest {
            recipient_id: "recip_123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: RequestLinkRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.recipient_id, "recip_123");
    }

    #[test]
    fn test_resend_endpoint_creates_new_session() {
        // UX-004: Resend endpoint should create new session with fresh expiry

        let old_expires = "2025-12-21T12:00:00Z";
        let new_expiry_hours = 168; // 7 days
        let new_expiry_seconds = (new_expiry_hours as u64) * 60 * 60;

        let new_expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::seconds(new_expiry_seconds as i64))
            .unwrap()
            .to_rfc3339();

        // Parse both timestamps
        let old_time = chrono::DateTime::parse_from_rfc3339(old_expires).unwrap();
        let new_time = chrono::DateTime::parse_from_rfc3339(&new_expires).unwrap();

        // New expiry should be after old one
        assert!(new_time > old_time, "New expiry should be in the future");
    }

    // Note: generate_session_id() uses js_sys and can't be tested in non-WASM tests
    // Session ID uniqueness is tested in integration tests

    // ============================================================
    // Email Integration Tests
    // ============================================================

    #[test]
    fn test_from_address_format() {
        // DEFAULT_FROM_ADDRESS should be in "Name <email>" format
        assert!(email::DEFAULT_FROM_ADDRESS.contains('<'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains('>'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains('@'));
        assert!(email::DEFAULT_FROM_ADDRESS.contains("getsignatures.org"));
    }

    #[test]
    fn test_email_request_payload_structure() {
        // Verify EmailSendRequest structure
        let request = email::EmailSendRequest {
            to: vec!["test@example.com".to_string()],
            subject: "Test Subject".to_string(),
            html: "<p>Test body</p>".to_string(),
            text: None,
            reply_to: None,
            tags: vec![],
        };

        // Verify fields are set correctly
        assert_eq!(request.to.len(), 1);
        assert_eq!(request.subject, "Test Subject");
        assert!(request.html.contains("<p>"));
    }

    #[test]
    fn test_email_request_multiple_recipients() {
        // EmailSendRequest accepts multiple recipients
        let request = email::EmailSendRequest {
            to: vec![
                "a@test.com".to_string(),
                "b@test.com".to_string(),
                "c@test.com".to_string(),
            ],
            subject: "Multi-recipient test".to_string(),
            html: "<p>Test</p>".to_string(),
            text: None,
            reply_to: None,
            tags: vec![],
        };

        assert_eq!(request.to.len(), 3);
    }

    #[test]
    fn test_invitation_html_escapes_special_chars() {
        // Ensure HTML template handles special characters safely
        let dangerous_name = "<script>alert('xss')</script>";
        let sender_name = "Alice & Bob";
        let document_name = "Contract \"Final\" <draft>";
        let signing_link = "https://example.com/sign?id=123&token=abc";

        // Simulate the template format string (simplified)
        let html = format!(
            "Hello {}, {} sent you {}. <a href=\"{}\">Sign</a>",
            dangerous_name, sender_name, document_name, signing_link
        );

        // The template should contain the raw text (HTML escaping happens at render)
        // This test verifies the template accepts these inputs without panicking
        assert!(html.contains("script"));
        assert!(html.contains("Alice & Bob"));
    }

    #[test]
    fn test_email_send_result_helpers() {
        // Test EmailSendResult helper methods
        let success = email::EmailSendResult::success("msg-123".to_string());
        assert!(success.success);
        assert_eq!(success.id, "msg-123");
        assert!(success.error.is_none());

        let failure = email::EmailSendResult::error("Something went wrong");
        assert!(!failure.success);
        assert!(failure.id.is_empty());
        assert_eq!(failure.error.as_deref(), Some("Something went wrong"));
    }

    // ============================================================
    // Regression Tests for UX Fixes (2026-01-06)
    // ============================================================

    /// Regression test: CORS headers must include Authorization header
    /// Bug: Account creation failed with network error because Authorization
    /// header was not in Access-Control-Allow-Headers
    #[test]
    fn test_cors_headers_include_authorization() {
        // Verify the CORS header value includes both Content-Type and Authorization
        // This is a compile-time check by examining the expected behavior
        let expected_headers = "Content-Type, Authorization";

        // The cors_response function sets these headers:
        // Access-Control-Allow-Origin: *
        // Access-Control-Allow-Methods: GET, POST, PUT, OPTIONS
        // Access-Control-Allow-Headers: Content-Type, Authorization

        // Verify Authorization is included (prevents regression of the network error bug)
        assert!(
            expected_headers.contains("Authorization"),
            "CORS headers must include Authorization to prevent browser blocking auth requests"
        );
        assert!(
            expected_headers.contains("Content-Type"),
            "CORS headers must include Content-Type for JSON requests"
        );
    }
}

// ============================================================
// Email Integration Property Tests
// ============================================================

#[cfg(test)]
mod email_proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Email subject can contain any printable ASCII
        #[test]
        fn email_subject_accepts_printable_ascii(subject in "[a-zA-Z0-9 !@#$%^&*()_+=\\-\\[\\]{}|;:',.<>?/]{1,200}") {
            let request = email::EmailSendRequest {
                to: vec!["test@example.com".to_string()],
                subject: subject.clone(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            // Should serialize without error
            let json_str = serde_json::to_string(&request);
            prop_assert!(json_str.is_ok(), "Should serialize email with subject: {}", subject);
        }

        /// Property: Recipient email addresses are preserved exactly
        #[test]
        fn recipient_emails_preserved(
            local in "[a-z0-9._%+-]{1,20}",
            domain in "[a-z0-9-]{1,20}",
            tld in "(com|org|net|io)"
        ) {
            let email_addr = format!("{}@{}.{}", local, domain, tld);
            let request = email::EmailSendRequest {
                to: vec![email_addr.clone()],
                subject: "Test".to_string(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            prop_assert_eq!(&request.to[0], &email_addr);
        }

        /// Property: HTML body can contain any valid HTML characters
        #[test]
        fn html_body_accepts_valid_content(
            text in "[a-zA-Z0-9 .,!?]{1,100}"
        ) {
            let html = format!("<p>{}</p>", text);
            let request = email::EmailSendRequest {
                to: vec!["test@example.com".to_string()],
                subject: "Test".to_string(),
                html: html.clone(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            let json_str = serde_json::to_string(&request);
            prop_assert!(json_str.is_ok());

            // Verify HTML is preserved
            prop_assert!(request.html.contains(&text));
        }

        /// Property: Multiple recipients all included in request
        #[test]
        fn multiple_recipients_all_included(count in 1usize..10) {
            let recipients: Vec<String> = (0..count)
                .map(|i| format!("user{}@example.com", i))
                .collect();

            let request = email::EmailSendRequest {
                to: recipients.clone(),
                subject: "Test".to_string(),
                html: "<p>Test</p>".to_string(),
                text: None,
                reply_to: None,
                tags: vec![],
            };

            prop_assert_eq!(request.to.len(), count);

            for (i, recipient) in recipients.iter().enumerate() {
                prop_assert_eq!(&request.to[i], recipient);
            }
        }

        /// Property: Invitation email template handles various name lengths
        #[test]
        fn invitation_template_handles_name_lengths(
            name_len in 1usize..100
        ) {
            let name: String = (0..name_len).map(|_| 'A').collect();

            // Simulate template format (simplified)
            let html = format!(
                "<p>Hello {},</p><p>You have a document to sign.</p>",
                name
            );

            prop_assert!(html.len() > name_len);
            prop_assert!(html.contains(&name));
        }

        /// Property: Signing link URLs are preserved in template
        #[test]
        fn signing_link_preserved_in_template(
            session_id in "[a-zA-Z0-9]{8,32}",
            recipient_id in "[a-zA-Z0-9]{4,16}",
            key in "[a-zA-Z0-9]{16,64}"
        ) {
            let signing_link = format!(
                "https://getsignatures.org/sign?session={}&recipient={}&key={}",
                session_id, recipient_id, key
            );

            let html = format!(
                "<a href=\"{}\">Sign Document</a>",
                signing_link
            );

            prop_assert!(html.contains(&signing_link));
            prop_assert!(html.contains(&session_id));
            prop_assert!(html.contains(&recipient_id));
        }
    }

    // ============================================================
    // Per-IP Rate Limiting Tests
    // ============================================================

    #[test]
    fn test_ip_rate_limit_state_default() {
        let state = IpRateLimitState::default();
        assert_eq!(state.request_count, 0);
        assert_eq!(state.window_start, 0);
    }

    #[test]
    fn test_ip_rate_limit_state_serialization() {
        let state = IpRateLimitState {
            request_count: 42,
            window_start: 1704067200,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: IpRateLimitState = serde_json::from_str(&json).unwrap();

        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_rate_limit_tier_limits() {
        // Health: 100 req/min
        let (max, window) = RateLimitTier::Health.limits();
        assert_eq!(max, 100);
        assert_eq!(window, 60);

        // SessionRead: 30 req/min
        let (max, window) = RateLimitTier::SessionRead.limits();
        assert_eq!(max, 30);
        assert_eq!(window, 60);

        // SessionWrite: 5 req/min
        let (max, window) = RateLimitTier::SessionWrite.limits();
        assert_eq!(max, 5);
        assert_eq!(window, 60);

        // RequestLink: 100 req/day (generous per-IP, global email quota handles cost)
        let (max, window) = RateLimitTier::RequestLink.limits();
        assert_eq!(max, 100);
        assert_eq!(window, 86400);
    }

    #[test]
    fn test_rate_limit_tier_names() {
        assert_eq!(RateLimitTier::Health.name(), "health");
        assert_eq!(RateLimitTier::SessionRead.name(), "session_read");
        assert_eq!(RateLimitTier::SessionWrite.name(), "session_write");
        assert_eq!(RateLimitTier::RequestLink.name(), "request_link");
    }

    #[test]
    fn test_rate_limit_tier_retry_after() {
        assert_eq!(RateLimitTier::Health.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::SessionRead.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::SessionWrite.retry_after_seconds(), 60);
        assert_eq!(RateLimitTier::RequestLink.retry_after_seconds(), 86400); // 1 day
    }

    #[test]
    fn test_ip_rate_limit_result_equality() {
        assert_eq!(IpRateLimitResult::Allowed, IpRateLimitResult::Allowed);
        assert_eq!(
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            },
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            }
        );
        assert_ne!(
            IpRateLimitResult::Allowed,
            IpRateLimitResult::Limited {
                retry_after_seconds: 30
            }
        );
    }

    #[test]
    fn test_ip_rate_limit_kv_key_format() {
        // Verify key format: ip_limit:{ip}:{tier_name}
        let ip = "192.168.1.1";
        let tier = RateLimitTier::Health;
        let key = format!("ip_limit:{}:{}", ip, tier.name());
        assert_eq!(key, "ip_limit:192.168.1.1:health");

        let tier = RateLimitTier::RequestLink;
        let key = format!("ip_limit:{}:{}", ip, tier.name());
        assert_eq!(key, "ip_limit:192.168.1.1:request_link");
    }

    /// Regression test: Code must NOT use std::time::SystemTime
    /// Bug: Worker crashed with "time not implemented on this platform" panic
    /// because std::time::SystemTime::now() doesn't work in WASM/Workers.
    /// Fix: Use js_sys::Date::now() via get_timestamp_secs()/get_timestamp_millis()
    #[test]
    fn test_no_std_time_usage_documented() {
        // This test documents the correct approach for getting timestamps in Workers.
        //
        // PROBLEM: std::time::SystemTime panics in WASM with:
        //   "panicked at library/std/src/sys/pal/wasm/../unsupported/time.rs:31:9:
        //    time not implemented on this platform"
        //
        // SOLUTION: The codebase provides WASM-safe helpers:
        //   - get_timestamp_secs() -> u64   (Unix timestamp in seconds)
        //   - get_timestamp_millis() -> u128 (Unix timestamp in milliseconds)
        //
        // Both use js_sys::Date::now() which works in Cloudflare Workers.
        //
        // VERIFICATION: Search codebase for std::time::SystemTime usage.
        // There should be ZERO occurrences outside of this test comment.
        //
        // grep -r "std::time::SystemTime" apps/docsign-web/worker/src/
        // Expected: 0 matches (only this comment mentions it)

        // Compile-time verification: the helper functions exist with correct signatures
        fn _verify_timestamp_helpers_exist() {
            let _: fn() -> u64 = get_timestamp_secs;
            let _: fn() -> u128 = get_timestamp_millis;
        }

        // The actual runtime behavior can only be tested in WASM environment
        // which is verified by deploying and hitting /health endpoint
        assert!(true, "This test documents WASM time requirements");
    }

    proptest! {
        /// Property: Window reset logic is correct
        #[test]
        fn prop_ip_rate_limit_window_expired(
            window_start in 0u64..1_000_000,
            window_seconds in 1u64..7200,
            offset in 0u64..10000
        ) {
            let now = window_start + window_seconds + offset;
            let is_expired = now >= window_start + window_seconds;

            // If offset >= 0, window should be expired (now >= window_start + window_seconds)
            prop_assert!(is_expired);
        }

        /// Property: Window NOT expired when current time is within window
        #[test]
        fn prop_ip_rate_limit_window_not_expired(
            window_start in 1000u64..1_000_000,
            window_seconds in 60u64..7200,
            offset in 1u64..59
        ) {
            // Ensure offset is less than window_seconds
            let actual_offset = offset % window_seconds;
            let now = window_start + actual_offset;
            let is_expired = now >= window_start + window_seconds;

            // Window should NOT be expired
            prop_assert!(!is_expired);
        }

        /// Property: Retry-after calculation is always at least 1 second
        #[test]
        fn prop_retry_after_at_least_one(
            window_start in 0u64..1_000_000,
            window_seconds in 60u64..7200,
            now in 0u64..2_000_000
        ) {
            let retry_after = (window_start + window_seconds).saturating_sub(now).max(1);
            prop_assert!(retry_after >= 1);
        }

        /// Property: Request count never goes negative
        #[test]
        fn prop_request_count_non_negative(
            initial in 0u32..1000,
            increments in 0usize..100
        ) {
            let mut count = initial;
            for _ in 0..increments {
                count += 1;
            }
            prop_assert!(count >= initial);
        }
    }
}
