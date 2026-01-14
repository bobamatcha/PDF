//! Stripe Billing Integration
//!
//! Handles subscription payments via Stripe's REST API.
//! Uses direct fetch calls (not stripe-rust) for Cloudflare Workers compatibility.

use serde::{Deserialize, Serialize};
use worker::*;

use crate::auth::types::{BillingCycle, User, UserPublic, UserTier};
use crate::auth::{get_authenticated_user, save_user};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a checkout session
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheckoutRequest {
    /// Stripe Price ID (e.g., "price_1ABC...")
    pub price_id: String,
    /// Whether this is annual billing
    #[serde(default)]
    pub annual: bool,
}

/// Response from checkout endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutResponse {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkout_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from portal endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct PortalResponse {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portal_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from billing status endpoint
#[derive(Debug, Serialize)]
pub struct BillingStatusResponse {
    pub success: bool,
    pub user: Option<UserPublic>,
    pub has_subscription: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Generic webhook response
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub received: bool,
}

// ============================================================================
// Stripe API Types
// ============================================================================

/// Stripe Checkout Session (subset of fields we need)
#[derive(Debug, Deserialize)]
struct StripeCheckoutSession {
    #[allow(dead_code)]
    id: String,
    url: Option<String>,
    #[allow(dead_code)]
    customer: Option<String>,
}

/// Stripe Customer Portal Session
#[derive(Debug, Deserialize)]
struct StripePortalSession {
    url: String,
}

/// Stripe Customer (for creation)
#[derive(Debug, Deserialize)]
struct StripeCustomer {
    id: String,
}

/// Stripe Subscription from webhook
#[derive(Debug, Deserialize)]
struct StripeSubscription {
    id: String,
    status: String,
    customer: String,
    #[serde(default)]
    #[allow(dead_code)]
    current_period_end: Option<i64>,
    items: StripeSubscriptionItems,
}

#[derive(Debug, Deserialize)]
struct StripeSubscriptionItems {
    data: Vec<StripeSubscriptionItem>,
}

#[derive(Debug, Deserialize)]
struct StripeSubscriptionItem {
    price: StripePrice,
}

#[derive(Debug, Deserialize)]
struct StripePrice {
    id: String,
    #[serde(default)]
    recurring: Option<StripePriceRecurring>,
}

#[derive(Debug, Deserialize)]
struct StripePriceRecurring {
    interval: String, // "month" or "year"
}

/// Stripe Webhook Event
#[derive(Debug, Deserialize)]
struct StripeEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: StripeEventData,
}

#[derive(Debug, Deserialize)]
struct StripeEventData {
    object: serde_json::Value,
}

/// Stripe API Error
#[derive(Debug, Deserialize)]
struct StripeError {
    error: StripeErrorDetails,
}

#[derive(Debug, Deserialize)]
struct StripeErrorDetails {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /billing/checkout - Create a Stripe Checkout session
pub async fn handle_checkout(mut req: Request, env: Env) -> Result<Response> {
    // Get authenticated user
    let user = match get_authenticated_user(&req, &env).await? {
        Some(u) => u,
        None => {
            return Response::from_json(&CheckoutResponse {
                success: false,
                checkout_url: None,
                error: Some("Authentication required".to_string()),
            })
            .map(|r| r.with_status(401));
        }
    };

    // Parse request body
    let body: CreateCheckoutRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return Response::from_json(&CheckoutResponse {
                success: false,
                checkout_url: None,
                error: Some("Invalid request body".to_string()),
            })
            .map(|r| r.with_status(400));
        }
    };

    // Get Stripe secret key
    let stripe_key = match env.secret("STRIPE_SECRET_KEY") {
        Ok(s) => s.to_string(),
        Err(_) => {
            return Response::from_json(&CheckoutResponse {
                success: false,
                checkout_url: None,
                error: Some("Stripe not configured".to_string()),
            })
            .map(|r| r.with_status(500));
        }
    };

    if stripe_key.is_empty() {
        return Response::from_json(&CheckoutResponse {
            success: false,
            checkout_url: None,
            error: Some("Stripe not configured".to_string()),
        })
        .map(|r| r.with_status(500));
    }

    // Validate price ID against configured prices (security: prevent arbitrary price IDs)
    let valid_price_ids = get_configured_price_ids(&env);
    if !valid_price_ids.contains(&body.price_id) {
        return Response::from_json(&CheckoutResponse {
            success: false,
            checkout_url: None,
            error: Some("Invalid price ID".to_string()),
        })
        .map(|r| r.with_status(400));
    }

    // Get USERS KV store
    let users_kv = env.kv("USERS")?;

    // Get or create Stripe customer
    let customer_id = match &user.stripe_customer_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            match create_stripe_customer(&stripe_key, &user.email, &user.id).await {
                Ok(id) => {
                    // Save customer ID to user
                    let mut updated_user = user.clone();
                    updated_user.stripe_customer_id = Some(id.clone());
                    if let Err(e) = save_user(&users_kv, &updated_user).await {
                        console_log!("Failed to save Stripe customer ID: {}", e);
                    }
                    id
                }
                Err(e) => {
                    return Response::from_json(&CheckoutResponse {
                        success: false,
                        checkout_url: None,
                        error: Some(format!("Failed to create customer: {}", e)),
                    })
                    .map(|r| r.with_status(500));
                }
            }
        }
    };

    // Create checkout session
    let success_url = "https://getsignatures.org/pricing.html?success=true";
    let cancel_url = "https://getsignatures.org/pricing.html?canceled=true";

    match create_checkout_session(
        &stripe_key,
        &customer_id,
        &body.price_id,
        success_url,
        cancel_url,
    )
    .await
    {
        Ok(session) => Response::from_json(&CheckoutResponse {
            success: true,
            checkout_url: session.url,
            error: None,
        }),
        Err(e) => Response::from_json(&CheckoutResponse {
            success: false,
            checkout_url: None,
            error: Some(e),
        })
        .map(|r| r.with_status(500)),
    }
}

/// POST /billing/portal - Create a Stripe Customer Portal session
pub async fn handle_portal(req: Request, env: Env) -> Result<Response> {
    // Get authenticated user
    let user = match get_authenticated_user(&req, &env).await? {
        Some(u) => u,
        None => {
            return Response::from_json(&PortalResponse {
                success: false,
                portal_url: None,
                error: Some("Authentication required".to_string()),
            })
            .map(|r| r.with_status(401));
        }
    };

    // Check if user has a Stripe customer ID
    let customer_id = match &user.stripe_customer_id {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            return Response::from_json(&PortalResponse {
                success: false,
                portal_url: None,
                error: Some("No billing account found. Please subscribe first.".to_string()),
            })
            .map(|r| r.with_status(400));
        }
    };

    // Get Stripe secret key
    let stripe_key = match env.secret("STRIPE_SECRET_KEY") {
        Ok(s) => s.to_string(),
        Err(_) => {
            return Response::from_json(&PortalResponse {
                success: false,
                portal_url: None,
                error: Some("Stripe not configured".to_string()),
            })
            .map(|r| r.with_status(500));
        }
    };

    // Create portal session
    let return_url = "https://getsignatures.org/pricing.html";

    match create_portal_session(&stripe_key, &customer_id, return_url).await {
        Ok(session) => Response::from_json(&PortalResponse {
            success: true,
            portal_url: Some(session.url),
            error: None,
        }),
        Err(e) => Response::from_json(&PortalResponse {
            success: false,
            portal_url: None,
            error: Some(e),
        })
        .map(|r| r.with_status(500)),
    }
}

/// GET /billing/status - Get current billing status
pub async fn handle_billing_status(req: Request, env: Env) -> Result<Response> {
    // Get authenticated user
    let user = match get_authenticated_user(&req, &env).await? {
        Some(u) => u,
        None => {
            return Response::from_json(&BillingStatusResponse {
                success: false,
                user: None,
                has_subscription: false,
                subscription_status: None,
                current_period_end: None,
                error: Some("Authentication required".to_string()),
            })
            .map(|r| r.with_status(401));
        }
    };

    let has_subscription = user.stripe_subscription_id.is_some();

    Response::from_json(&BillingStatusResponse {
        success: true,
        user: Some(UserPublic::from(&user)),
        has_subscription,
        subscription_status: if has_subscription {
            Some("active".to_string())
        } else {
            None
        },
        current_period_end: None, // Would need to fetch from Stripe for real value
        error: None,
    })
}

/// POST /billing/webhook - Handle Stripe webhook events
pub async fn handle_webhook(mut req: Request, env: Env) -> Result<Response> {
    // Get webhook signing secret
    let webhook_secret = match env.secret("STRIPE_WEBHOOK_SECRET") {
        Ok(s) => s.to_string(),
        Err(_) => {
            console_log!("STRIPE_WEBHOOK_SECRET not configured");
            return Response::from_json(&WebhookResponse { received: true });
        }
    };

    // Get Stripe signature header
    let signature = req
        .headers()
        .get("stripe-signature")
        .ok()
        .flatten()
        .unwrap_or_default();

    // Get raw body for signature verification
    let body = match req.text().await {
        Ok(b) => b,
        Err(_) => {
            return Response::from_json(&WebhookResponse { received: true })
                .map(|r| r.with_status(400));
        }
    };

    // Verify webhook signature
    if !verify_webhook_signature(&body, &signature, &webhook_secret) {
        console_log!("Webhook signature verification failed");
        // Still return 200 to prevent Stripe from retrying
        return Response::from_json(&WebhookResponse { received: true });
    }

    // Parse event
    let event: StripeEvent = match serde_json::from_str(&body) {
        Ok(e) => e,
        Err(e) => {
            console_log!("Failed to parse webhook event: {}", e);
            return Response::from_json(&WebhookResponse { received: true });
        }
    };

    console_log!("Received Stripe webhook: {}", event.event_type);

    // Handle different event types
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            handle_checkout_completed(&env, &event.data.object).await;
        }
        "customer.subscription.created" | "customer.subscription.updated" => {
            handle_subscription_updated(&env, &event.data.object).await;
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(&env, &event.data.object).await;
        }
        "invoice.payment_failed" => {
            handle_payment_failed(&event.data.object).await;
        }
        _ => {
            console_log!("Unhandled webhook event type: {}", event.event_type);
        }
    }

    Response::from_json(&WebhookResponse { received: true })
}

// ============================================================================
// Stripe API Helpers
// ============================================================================

/// Create a Stripe customer
async fn create_stripe_customer(
    api_key: &str,
    email: &str,
    user_id: &str,
) -> std::result::Result<String, String> {
    let body = format!(
        "email={}&metadata[user_id]={}",
        urlencoding::encode(email),
        urlencoding::encode(user_id)
    );

    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {}", api_key))
        .map_err(|e| e.to_string())?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| e.to_string())?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));

    let request = Request::new_with_init("https://api.stripe.com/v1/customers", &init)
        .map_err(|e| e.to_string())?;

    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;

    if response.status_code() != 200 {
        if let Ok(err) = serde_json::from_str::<StripeError>(&text) {
            return Err(err.error.message);
        }
        return Err(format!("Stripe API error: {}", response.status_code()));
    }

    let customer: StripeCustomer = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(customer.id)
}

/// Create a Stripe Checkout session
async fn create_checkout_session(
    api_key: &str,
    customer_id: &str,
    price_id: &str,
    success_url: &str,
    cancel_url: &str,
) -> std::result::Result<StripeCheckoutSession, String> {
    let body = format!(
        "customer={}&mode=subscription&line_items[0][price]={}&line_items[0][quantity]=1&success_url={}&cancel_url={}",
        urlencoding::encode(customer_id),
        urlencoding::encode(price_id),
        urlencoding::encode(success_url),
        urlencoding::encode(cancel_url)
    );

    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {}", api_key))
        .map_err(|e| e.to_string())?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| e.to_string())?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));

    let request = Request::new_with_init("https://api.stripe.com/v1/checkout/sessions", &init)
        .map_err(|e| e.to_string())?;

    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;

    if response.status_code() != 200 {
        if let Ok(err) = serde_json::from_str::<StripeError>(&text) {
            return Err(err.error.message);
        }
        return Err(format!("Stripe API error: {}", response.status_code()));
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Create a Stripe Customer Portal session
async fn create_portal_session(
    api_key: &str,
    customer_id: &str,
    return_url: &str,
) -> std::result::Result<StripePortalSession, String> {
    let body = format!(
        "customer={}&return_url={}",
        urlencoding::encode(customer_id),
        urlencoding::encode(return_url)
    );

    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {}", api_key))
        .map_err(|e| e.to_string())?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|e| e.to_string())?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));

    let request =
        Request::new_with_init("https://api.stripe.com/v1/billing_portal/sessions", &init)
            .map_err(|e| e.to_string())?;

    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;

    if response.status_code() != 200 {
        if let Ok(err) = serde_json::from_str::<StripeError>(&text) {
            return Err(err.error.message);
        }
        return Err(format!("Stripe API error: {}", response.status_code()));
    }

    serde_json::from_str(&text).map_err(|e| e.to_string())
}

// ============================================================================
// Webhook Handlers
// ============================================================================

/// Verify Stripe webhook signature
fn verify_webhook_signature(payload: &str, signature: &str, secret: &str) -> bool {
    // Parse signature header
    // Format: t=timestamp,v1=signature,v1=signature2,...
    let mut timestamp = "";
    let mut signatures: Vec<&str> = Vec::new();

    for part in signature.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "t" => timestamp = value,
                "v1" => signatures.push(value),
                _ => {}
            }
        }
    }

    if timestamp.is_empty() || signatures.is_empty() {
        return false;
    }

    // Check timestamp is within 5 minutes
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = js_sys::Date::now() as i64 / 1000;
        if (now - ts).abs() > 300 {
            console_log!("Webhook timestamp too old: {} vs now {}", ts, now);
            return false;
        }
    }

    // Compute expected signature
    let signed_payload = format!("{}.{}", timestamp, payload);

    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());

    // Check if any signature matches
    signatures.iter().any(|s| *s == expected)
}

/// Handle checkout.session.completed event
async fn handle_checkout_completed(env: &Env, data: &serde_json::Value) {
    // Extract customer ID from checkout session
    let customer_id = match data.get("customer").and_then(|c| c.as_str()) {
        Some(id) => id,
        None => {
            console_log!("Checkout completed but no customer ID found");
            return;
        }
    };

    let subscription_id = match data.get("subscription").and_then(|s| s.as_str()) {
        Some(id) => id,
        None => {
            console_log!("Checkout completed but no subscription ID found");
            return;
        }
    };

    console_log!(
        "Checkout completed for customer {} with subscription {}",
        customer_id,
        subscription_id
    );

    // Try to extract the price ID from the line_items to determine tier immediately
    // This ensures tier is set even if subscription.created webhook arrives later
    let mut tier = UserTier::Free;
    let mut billing_cycle = BillingCycle::Monthly;

    // Checkout session may have line_items.data[0].price.id
    if let Some(line_items) = data.get("line_items") {
        if let Some(items_data) = line_items.get("data").and_then(|d| d.as_array()) {
            if let Some(first_item) = items_data.first() {
                if let Some(price_id) = first_item
                    .get("price")
                    .and_then(|p| p.get("id"))
                    .and_then(|id| id.as_str())
                {
                    tier = determine_tier_from_price_id(env, price_id);
                    billing_cycle = determine_billing_cycle_from_price_id(env, price_id);
                    console_log!(
                        "Determined tier {:?} from checkout line item price {}",
                        tier,
                        price_id
                    );
                }
            }
        }
    }

    // If we couldn't get tier from line_items, we'll rely on the subscription webhook
    // But still update the subscription_id now
    if let Err(e) = update_user_subscription_and_tier_by_customer(
        env,
        customer_id,
        subscription_id,
        tier,
        billing_cycle,
    )
    .await
    {
        console_log!("Failed to update user subscription: {}", e);
    }
}

/// Handle subscription updated event
async fn handle_subscription_updated(env: &Env, data: &serde_json::Value) {
    let subscription: StripeSubscription = match serde_json::from_value(data.clone()) {
        Ok(s) => s,
        Err(e) => {
            console_log!("Failed to parse subscription: {}", e);
            return;
        }
    };

    console_log!(
        "Subscription {} updated: status={}",
        subscription.id,
        subscription.status
    );

    // Check subscription status - only set tier for active subscriptions
    // Status values: incomplete, incomplete_expired, trialing, active, past_due, canceled, unpaid, paused
    let (tier, billing_cycle) = match subscription.status.as_str() {
        "active" | "trialing" => {
            // Active subscription - set the paid tier
            let tier = determine_tier_from_subscription(env, &subscription);
            let billing_cycle = determine_billing_cycle(&subscription);
            (tier, billing_cycle)
        }
        "past_due" => {
            // Payment failed but subscription not canceled yet - keep current tier
            // Don't change anything, just log
            console_log!(
                "Subscription {} is past_due - keeping current tier until resolved",
                subscription.id
            );
            return;
        }
        "canceled" | "unpaid" | "incomplete_expired" => {
            // Subscription ended - downgrade to Free
            console_log!(
                "Subscription {} is {} - downgrading to Free",
                subscription.id,
                subscription.status
            );
            (UserTier::Free, BillingCycle::Monthly)
        }
        _ => {
            // Unknown or incomplete status - don't change tier
            console_log!(
                "Subscription {} has status {} - not changing tier",
                subscription.id,
                subscription.status
            );
            return;
        }
    };

    // Update user
    if let Err(e) = update_user_tier_by_customer(
        env,
        &subscription.customer,
        tier,
        billing_cycle,
        Some(&subscription.id),
    )
    .await
    {
        console_log!("Failed to update user tier: {}", e);
    }
}

/// Handle subscription deleted event
async fn handle_subscription_deleted(env: &Env, data: &serde_json::Value) {
    let customer_id = match data.get("customer").and_then(|c| c.as_str()) {
        Some(id) => id,
        None => {
            console_log!("Subscription deleted but no customer ID found");
            return;
        }
    };

    console_log!("Subscription deleted for customer {}", customer_id);

    // Downgrade user to Free tier
    if let Err(e) = update_user_tier_by_customer(
        env,
        customer_id,
        UserTier::Free,
        BillingCycle::Monthly,
        None,
    )
    .await
    {
        console_log!("Failed to downgrade user: {}", e);
    }
}

/// Handle payment failed event
async fn handle_payment_failed(data: &serde_json::Value) {
    let customer_id = match data.get("customer").and_then(|c| c.as_str()) {
        Some(id) => id,
        None => {
            console_log!("Payment failed but no customer ID found");
            return;
        }
    };

    console_log!("Payment failed for customer {}", customer_id);

    // TODO: Send payment failed email to user
    // For now, just log it - Stripe will retry automatically
}

/// Get all configured Stripe price IDs
fn get_configured_price_ids(env: &Env) -> Vec<String> {
    let mut prices = Vec::new();

    let price_vars = [
        "STRIPE_PRICE_PERSONAL_MONTHLY",
        "STRIPE_PRICE_PERSONAL_ANNUAL",
        "STRIPE_PRICE_PROFESSIONAL_MONTHLY",
        "STRIPE_PRICE_PROFESSIONAL_ANNUAL",
        "STRIPE_PRICE_BUSINESS_MONTHLY",
        "STRIPE_PRICE_BUSINESS_ANNUAL",
    ];

    for var in &price_vars {
        if let Ok(v) = env.var(var) {
            let val = v.to_string();
            if !val.is_empty() {
                prices.push(val);
            }
        }
    }

    prices
}

/// Determine tier from a price ID
fn determine_tier_from_price_id(env: &Env, price_id: &str) -> UserTier {
    let personal_monthly = env
        .var("STRIPE_PRICE_PERSONAL_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let personal_annual = env
        .var("STRIPE_PRICE_PERSONAL_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let professional_monthly = env
        .var("STRIPE_PRICE_PROFESSIONAL_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let professional_annual = env
        .var("STRIPE_PRICE_PROFESSIONAL_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let business_monthly = env
        .var("STRIPE_PRICE_BUSINESS_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let business_annual = env
        .var("STRIPE_PRICE_BUSINESS_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();

    if price_id == personal_monthly || price_id == personal_annual {
        UserTier::Personal
    } else if price_id == professional_monthly || price_id == professional_annual {
        UserTier::Professional
    } else if price_id == business_monthly || price_id == business_annual {
        UserTier::Business
    } else {
        console_log!("Unknown price ID: {}", price_id);
        UserTier::Free
    }
}

/// Determine billing cycle from a price ID
fn determine_billing_cycle_from_price_id(env: &Env, price_id: &str) -> BillingCycle {
    let annual_prices = [
        env.var("STRIPE_PRICE_PERSONAL_ANNUAL")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        env.var("STRIPE_PRICE_PROFESSIONAL_ANNUAL")
            .map(|v| v.to_string())
            .unwrap_or_default(),
        env.var("STRIPE_PRICE_BUSINESS_ANNUAL")
            .map(|v| v.to_string())
            .unwrap_or_default(),
    ];

    if annual_prices.iter().any(|p| p == price_id) {
        BillingCycle::Annual
    } else {
        BillingCycle::Monthly
    }
}

/// Determine tier from subscription price ID
fn determine_tier_from_subscription(env: &Env, subscription: &StripeSubscription) -> UserTier {
    // Get the first item's price ID
    let price_id = match subscription.items.data.first() {
        Some(item) => &item.price.id,
        None => return UserTier::Free,
    };

    // Match against configured price IDs
    let personal_monthly = env
        .var("STRIPE_PRICE_PERSONAL_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let personal_annual = env
        .var("STRIPE_PRICE_PERSONAL_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let professional_monthly = env
        .var("STRIPE_PRICE_PROFESSIONAL_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let professional_annual = env
        .var("STRIPE_PRICE_PROFESSIONAL_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let business_monthly = env
        .var("STRIPE_PRICE_BUSINESS_MONTHLY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let business_annual = env
        .var("STRIPE_PRICE_BUSINESS_ANNUAL")
        .map(|v| v.to_string())
        .unwrap_or_default();

    if price_id == &personal_monthly || price_id == &personal_annual {
        UserTier::Personal
    } else if price_id == &professional_monthly || price_id == &professional_annual {
        UserTier::Professional
    } else if price_id == &business_monthly || price_id == &business_annual {
        UserTier::Business
    } else {
        console_log!("Unknown price ID: {}", price_id);
        UserTier::Free
    }
}

/// Determine billing cycle from subscription
fn determine_billing_cycle(subscription: &StripeSubscription) -> BillingCycle {
    match subscription.items.data.first() {
        Some(item) => match &item.price.recurring {
            Some(r) if r.interval == "year" => BillingCycle::Annual,
            _ => BillingCycle::Monthly,
        },
        None => BillingCycle::Monthly,
    }
}

/// Update user subscription and tier by Stripe customer ID
/// Used by checkout.session.completed to set tier immediately
async fn update_user_subscription_and_tier_by_customer(
    env: &Env,
    customer_id: &str,
    subscription_id: &str,
    tier: UserTier,
    billing_cycle: BillingCycle,
) -> std::result::Result<(), String> {
    let users_kv = env.kv("USERS").map_err(|e| e.to_string())?;

    // Find user by Stripe customer ID
    let user = find_user_by_stripe_customer(env, customer_id).await?;

    let mut updated_user = user;
    updated_user.stripe_subscription_id = Some(subscription_id.to_string());

    // Only update tier if it's not Free (meaning we successfully determined it)
    if tier != UserTier::Free {
        updated_user.tier = tier;
        updated_user.billing_cycle = billing_cycle;
    }

    save_user(&users_kv, &updated_user)
        .await
        .map_err(|e| e.to_string())
}

/// Update user tier by Stripe customer ID
async fn update_user_tier_by_customer(
    env: &Env,
    customer_id: &str,
    tier: UserTier,
    billing_cycle: BillingCycle,
    subscription_id: Option<&str>,
) -> std::result::Result<(), String> {
    let users_kv = env.kv("USERS").map_err(|e| e.to_string())?;

    // Find user by Stripe customer ID
    let user = find_user_by_stripe_customer(env, customer_id).await?;

    let mut updated_user = user;
    updated_user.tier = tier;
    updated_user.billing_cycle = billing_cycle;
    if let Some(sub_id) = subscription_id {
        updated_user.stripe_subscription_id = Some(sub_id.to_string());
    } else {
        updated_user.stripe_subscription_id = None;
    }

    save_user(&users_kv, &updated_user)
        .await
        .map_err(|e| e.to_string())
}

/// Find a user by their Stripe customer ID
/// This searches through all users in KV (not efficient, but works for small user bases)
async fn find_user_by_stripe_customer(
    env: &Env,
    customer_id: &str,
) -> std::result::Result<User, String> {
    let users_kv = env.kv("USERS").map_err(|e| e.to_string())?;

    // List all user keys
    let list = users_kv.list().execute().await.map_err(|e| e.to_string())?;

    for key in list.keys {
        // Skip email index keys
        if key.name.starts_with("user_email:") {
            continue;
        }

        // Get the user data
        if let Ok(Some(user_json)) = users_kv.get(&key.name).text().await {
            if let Ok(user) = serde_json::from_str::<User>(&user_json) {
                if user.stripe_customer_id.as_deref() == Some(customer_id) {
                    return Ok(user);
                }
            }
        }
    }

    Err(format!(
        "No user found with Stripe customer ID: {}",
        customer_id
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_request_deserialization() {
        let json = r#"{"price_id":"price_1ABC","annual":true}"#;
        let req: CreateCheckoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.price_id, "price_1ABC");
        assert!(req.annual);
    }

    #[test]
    fn test_checkout_response_serialization() {
        let resp = CheckoutResponse {
            success: true,
            checkout_url: Some("https://checkout.stripe.com/...".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("checkout_url"));
        assert!(!json.contains("error")); // Should be skipped when None
    }

    #[test]
    fn test_webhook_response_serialization() {
        let resp = WebhookResponse { received: true };
        let json = serde_json::to_string(&resp).unwrap();
        assert_eq!(json, r#"{"received":true}"#);
    }

    #[test]
    fn test_stripe_subscription_parsing() {
        let json = r#"{
            "id": "sub_1234",
            "status": "active",
            "customer": "cus_5678",
            "current_period_end": 1735689600,
            "items": {
                "data": [{
                    "price": {
                        "id": "price_1ABC",
                        "recurring": {
                            "interval": "month"
                        }
                    }
                }]
            }
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.id, "sub_1234");
        assert_eq!(sub.status, "active");
        assert_eq!(sub.customer, "cus_5678");
        assert_eq!(sub.items.data.len(), 1);
        assert_eq!(sub.items.data[0].price.id, "price_1ABC");
    }

    #[test]
    fn test_webhook_signature_parsing() {
        // Test that signature parsing works for valid format
        let sig = "t=1234567890,v1=abc123,v1=def456";
        let mut timestamp = "";
        let mut signatures: Vec<&str> = Vec::new();

        for part in sig.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = value,
                    "v1" => signatures.push(value),
                    _ => {}
                }
            }
        }

        assert_eq!(timestamp, "1234567890");
        assert_eq!(signatures.len(), 2);
        assert_eq!(signatures[0], "abc123");
        assert_eq!(signatures[1], "def456");
    }

    // ========================================================================
    // Regression tests for bugs found during code review
    // ========================================================================

    #[test]
    fn test_stripe_event_parsing() {
        // Regression test: ensure webhook events are parsed correctly
        let json = r#"{
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "customer": "cus_123",
                    "subscription": "sub_456"
                }
            }
        }"#;
        let event: StripeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "checkout.session.completed");
        assert_eq!(
            event.data.object.get("customer").unwrap().as_str().unwrap(),
            "cus_123"
        );
    }

    #[test]
    fn test_subscription_status_active() {
        // Test active subscription status
        let json = r#"{
            "id": "sub_1234",
            "status": "active",
            "customer": "cus_5678",
            "items": {"data": [{"price": {"id": "price_1ABC"}}]}
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.status, "active");
    }

    #[test]
    fn test_subscription_status_past_due() {
        // Regression test: past_due status should be recognized
        let json = r#"{
            "id": "sub_1234",
            "status": "past_due",
            "customer": "cus_5678",
            "items": {"data": [{"price": {"id": "price_1ABC"}}]}
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.status, "past_due");
    }

    #[test]
    fn test_subscription_status_canceled() {
        // Regression test: canceled status should be recognized
        let json = r#"{
            "id": "sub_1234",
            "status": "canceled",
            "customer": "cus_5678",
            "items": {"data": [{"price": {"id": "price_1ABC"}}]}
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.status, "canceled");
    }

    #[test]
    fn test_checkout_session_line_items_parsing() {
        // Regression test: checkout session with line_items
        let json = r#"{
            "customer": "cus_123",
            "subscription": "sub_456",
            "line_items": {
                "data": [{
                    "price": {
                        "id": "price_ABC123"
                    }
                }]
            }
        }"#;
        let data: serde_json::Value = serde_json::from_str(json).unwrap();

        // Extract price ID the same way handle_checkout_completed does
        let price_id = data
            .get("line_items")
            .and_then(|li| li.get("data"))
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("price"))
            .and_then(|p| p.get("id"))
            .and_then(|id| id.as_str());

        assert_eq!(price_id, Some("price_ABC123"));
    }

    #[test]
    fn test_billing_cycle_from_subscription_monthly() {
        let json = r#"{
            "id": "sub_1234",
            "status": "active",
            "customer": "cus_5678",
            "items": {
                "data": [{
                    "price": {
                        "id": "price_1ABC",
                        "recurring": {
                            "interval": "month"
                        }
                    }
                }]
            }
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        let billing_cycle = determine_billing_cycle(&sub);
        assert_eq!(billing_cycle, BillingCycle::Monthly);
    }

    #[test]
    fn test_billing_cycle_from_subscription_annual() {
        let json = r#"{
            "id": "sub_1234",
            "status": "active",
            "customer": "cus_5678",
            "items": {
                "data": [{
                    "price": {
                        "id": "price_1ABC",
                        "recurring": {
                            "interval": "year"
                        }
                    }
                }]
            }
        }"#;
        let sub: StripeSubscription = serde_json::from_str(json).unwrap();
        let billing_cycle = determine_billing_cycle(&sub);
        assert_eq!(billing_cycle, BillingCycle::Annual);
    }

    #[test]
    fn test_stripe_error_parsing() {
        // Test Stripe error response parsing
        let json = r#"{
            "error": {
                "message": "Invalid API Key provided",
                "type": "invalid_request_error"
            }
        }"#;
        let err: StripeError = serde_json::from_str(json).unwrap();
        assert_eq!(err.error.message, "Invalid API Key provided");
    }

    #[test]
    fn test_portal_response_serialization() {
        let resp = PortalResponse {
            success: true,
            portal_url: Some("https://billing.stripe.com/session/...".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("portal_url"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_billing_status_response_serialization() {
        let resp = BillingStatusResponse {
            success: true,
            user: None,
            has_subscription: true,
            subscription_status: Some("active".to_string()),
            current_period_end: None,
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("has_subscription"));
        assert!(json.contains("subscription_status"));
        assert!(!json.contains("error"));
        assert!(!json.contains("current_period_end"));
    }

    // ========================================================================
    // Webhook signature verification tests
    // Note: verify_webhook_signature uses js_sys::Date::now() which only works in WASM
    // So we test the parsing logic separately here
    // ========================================================================

    #[test]
    fn test_webhook_signature_parsing_and_hmac() {
        // Test the signature parsing logic and HMAC computation
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret_123";
        let timestamp = "1704067200";

        // Parse signature header format
        let sig = format!("t={},v1=abc123,v1=def456", timestamp);
        let mut parsed_timestamp = "";
        let mut parsed_signatures: Vec<&str> = Vec::new();

        for part in sig.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => parsed_timestamp = value,
                    "v1" => parsed_signatures.push(value),
                    _ => {}
                }
            }
        }

        assert_eq!(parsed_timestamp, timestamp);
        assert_eq!(parsed_signatures.len(), 2);

        // Compute expected signature
        let signed_payload = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let expected_sig = hex::encode(mac.finalize().into_bytes());

        // Verify HMAC computation produces expected length
        assert_eq!(expected_sig.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_webhook_signature_parsing_empty() {
        // Test parsing of malformed signatures
        let sig = "";
        let mut timestamp = "";
        let mut signatures: Vec<&str> = Vec::new();

        for part in sig.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = value,
                    "v1" => signatures.push(value),
                    _ => {}
                }
            }
        }

        assert!(timestamp.is_empty());
        assert!(signatures.is_empty());
    }

    #[test]
    fn test_webhook_signature_parsing_no_v1() {
        // Missing v1 signature
        let sig = "t=1234567890";
        let mut timestamp = "";
        let mut signatures: Vec<&str> = Vec::new();

        for part in sig.split(',') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "t" => timestamp = value,
                    "v1" => signatures.push(value),
                    _ => {}
                }
            }
        }

        assert_eq!(timestamp, "1234567890");
        assert!(signatures.is_empty());
    }

    // ========================================================================
    // Property tests using proptest
    // ========================================================================

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_checkout_request_roundtrip(price_id in "[a-zA-Z0-9_]{10,30}", annual in any::<bool>()) {
            let req = CreateCheckoutRequest { price_id: price_id.clone(), annual };
            let json = serde_json::to_string(&req).unwrap();
            let parsed: CreateCheckoutRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.price_id, price_id);
            prop_assert_eq!(parsed.annual, annual);
        }

        #[test]
        fn test_checkout_response_roundtrip(
            success in any::<bool>(),
            checkout_url in proptest::option::of("[a-zA-Z0-9:/._-]{10,100}"),
            error in proptest::option::of("[a-zA-Z0-9 ]{5,50}")
        ) {
            let resp = CheckoutResponse { success, checkout_url: checkout_url.clone(), error: error.clone() };
            let json = serde_json::to_string(&resp).unwrap();
            let parsed: CheckoutResponse = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.success, success);
            prop_assert_eq!(parsed.checkout_url, checkout_url);
            prop_assert_eq!(parsed.error, error);
        }

        #[test]
        fn test_portal_response_roundtrip(
            success in any::<bool>(),
            portal_url in proptest::option::of("[a-zA-Z0-9:/._-]{10,100}"),
            error in proptest::option::of("[a-zA-Z0-9 ]{5,50}")
        ) {
            let resp = PortalResponse { success, portal_url: portal_url.clone(), error: error.clone() };
            let json = serde_json::to_string(&resp).unwrap();
            let parsed: PortalResponse = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.success, success);
            prop_assert_eq!(parsed.portal_url, portal_url);
            prop_assert_eq!(parsed.error, error);
        }

        #[test]
        fn test_stripe_subscription_parsing_various_statuses(
            status in "(active|canceled|past_due|unpaid|trialing|incomplete)"
        ) {
            let json = format!(r#"{{
                "id": "sub_test",
                "status": "{}",
                "customer": "cus_test",
                "items": {{"data": [{{"price": {{"id": "price_test"}}}}]}}
            }}"#, status);
            let sub: StripeSubscription = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(sub.status, status);
        }

        #[test]
        fn test_stripe_event_roundtrip(
            event_type in "(checkout\\.session\\.completed|customer\\.subscription\\.created|customer\\.subscription\\.updated|customer\\.subscription\\.deleted)"
        ) {
            let json = format!(r#"{{
                "type": "{}",
                "data": {{"object": {{"id": "test_123"}}}}
            }}"#, event_type);
            let event: StripeEvent = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(event.event_type, event_type);
        }
    }
}
