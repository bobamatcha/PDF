# Email Proxy Lambda

AWS Lambda email proxy with SES integration and automated deliverability best practices.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────┐     ┌───────────────┐
│ Cloudflare      │────▶│ API Gateway     │────▶│ Lambda      │────▶│ AWS SES       │
│ Worker          │     │ (HTTP API)      │     │ (Rust)      │     │               │
└─────────────────┘     └─────────────────┘     └─────────────┘     └───────────────┘
                                                      │                      │
                                                      ▼                      │
                                               ┌─────────────┐               │
                                               │ DynamoDB    │◀──────────────┘
                                               │ (suppression│    SNS notifications
                                               │  list)      │    (bounces/complaints)
                                               └─────────────┘
```

## How It Works

### Email Flow

1. **Request**: Cloudflare Worker sends POST to `/send` with email content
2. **Validation**: Lambda validates email format, checks suppression list
3. **Rate Limiting**: Governor rate limiter enforces per-second limits
4. **Warm-up Check**: If in warm-up period, daily limit is enforced
5. **Send**: Email sent via AWS SES v2 API
6. **Track**: SES returns message ID, Lambda records metrics

### Deliverability Features (Resend's Top 10 Tips Automated)

| Tip | Implementation |
|-----|----------------|
| 1. Authenticate domain (DKIM/SPF/DMARC) | Configured in AWS SES (see setup) |
| 2. Clean email list | Automatic suppression list (bounces/complaints) |
| 3. Warm up IP | 4-week graduated warm-up schedule |
| 4. Double opt-in | DocSign consent tracking |
| 5. Personalize emails | Template system with variables |
| 6. Unsubscribe link | List-Unsubscribe header auto-added |
| 7. Monitor reputation | Metrics tracking + health score |
| 8. Avoid spam words | Content scanner (optional) |
| 9. Relevant content | Application responsibility |
| 10. Test before send | `/validate` endpoint |

## API Endpoints

### POST /send

Send an email directly.

```json
{
  "from": "GetSignatures <noreply@getsignatures.org>",
  "to": ["recipient@example.com"],
  "subject": "Document ready for signature",
  "html": "<p>Hello...</p>",
  "text": "Hello...",
  "tags": [{"name": "type", "value": "signature_request"}]
}
```

Response:
```json
{
  "id": "0100018e-1234-5678-abcd-example",
  "queued_at": "2025-01-15T10:30:00Z",
  "status": "queued"
}
```

### POST /send/template

Send using a DocSign template.

```json
{
  "template": {
    "type": "signature_request",
    "signer_name": "John Doe",
    "signer_email": "john@example.com",
    "sender_name": "Jane Smith",
    "document_name": "Contract.pdf",
    "signing_url": "https://sign.getsignatures.org/abc123"
  }
}
```

### POST /notifications

SNS webhook for bounce/complaint processing.

### GET /health

Health check with warm-up status.

### GET /metrics

Reputation metrics (bounce rate, complaint rate, health score).

### POST /validate

Validate email without sending.

## Deployment

### Prerequisites

1. **AWS Account** with SES production access (exit sandbox)
2. **Verified domain** in SES
3. **IAM role** with SES permissions

### Setup AWS SES

```bash
# 1. Verify your domain in SES
aws ses verify-domain-identity --domain getsignatures.org

# 2. Add DKIM records to DNS (SES provides these)
aws ses get-identity-dkim-attributes --identities getsignatures.org

# 3. Add SPF record to DNS:
#    v=spf1 include:amazonses.com ~all

# 4. Add DMARC record to DNS:
#    _dmarc.getsignatures.org TXT "v=DMARC1; p=quarantine; rua=mailto:dmarc@getsignatures.org"

# 5. Create configuration set for tracking
aws sesv2 create-configuration-set --configuration-set-name docsign-transactional

# 6. Add SNS topic for bounces/complaints
aws sns create-topic --name docsign-email-notifications
aws sesv2 create-configuration-set-event-destination \
  --configuration-set-name docsign-transactional \
  --event-destination-name bounces \
  --event-destination 'Enabled=true,MatchingEventTypes=BOUNCE,COMPLAINT,DELIVERY,SnsDestination={TopicArn=arn:aws:sns:REGION:ACCOUNT:docsign-email-notifications}'
```

### Build & Deploy Lambda

```bash
# Install cargo-lambda
cargo install cargo-lambda

# Build for ARM64 (30% cheaper than x86)
cargo lambda build -p email-proxy --release --arm64

# Deploy
cargo lambda deploy email-proxy \
  --iam-role arn:aws:iam::ACCOUNT:role/email-proxy-lambda \
  --env FROM_DOMAIN=getsignatures.org \
  --env DEFAULT_FROM="GetSignatures <noreply@getsignatures.org>" \
  --env SES_CONFIGURATION_SET=docsign-transactional \
  --env WARM_UP_ENABLED=true \
  --env RATE_LIMIT_PER_SECOND=14
```

### Create API Gateway

```bash
# Create HTTP API
aws apigatewayv2 create-api \
  --name email-proxy-api \
  --protocol-type HTTP \
  --target arn:aws:lambda:REGION:ACCOUNT:function:email-proxy

# Add routes
aws apigatewayv2 create-route --api-id API_ID --route-key "POST /send"
aws apigatewayv2 create-route --api-id API_ID --route-key "POST /send/template"
aws apigatewayv2 create-route --api-id API_ID --route-key "GET /health"
aws apigatewayv2 create-route --api-id API_ID --route-key "GET /metrics"
```

### Subscribe SNS to Lambda

```bash
# Add permission
aws lambda add-permission \
  --function-name email-proxy \
  --statement-id sns-invoke \
  --action lambda:InvokeFunction \
  --principal sns.amazonaws.com \
  --source-arn arn:aws:sns:REGION:ACCOUNT:docsign-email-notifications

# Subscribe
aws sns subscribe \
  --topic-arn arn:aws:sns:REGION:ACCOUNT:docsign-email-notifications \
  --protocol lambda \
  --notification-endpoint arn:aws:lambda:REGION:ACCOUNT:function:email-proxy
```

## Integration with Cloudflare Worker

Update the DocSign worker to call the Lambda instead of Resend:

```javascript
// In apps/docsign-web/worker/src/lib.rs (or JS worker)
const EMAIL_API = "https://API_ID.execute-api.REGION.amazonaws.com";

async function sendSignatureRequest(recipient, document) {
  const response = await fetch(`${EMAIL_API}/send/template`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      template: {
        type: "signature_request",
        signer_name: recipient.name,
        signer_email: recipient.email,
        sender_name: document.senderName,
        document_name: document.name,
        signing_url: `https://sign.getsignatures.org/${document.sessionId}`,
      },
    }),
  });
  return response.json();
}
```

## Costs

| Component | Cost |
|-----------|------|
| SES | $0.10 per 1,000 emails |
| Lambda | ~$0.20 per 1M invocations (ARM64) |
| API Gateway | $1.00 per 1M requests |
| SNS | Free for Lambda delivery |

**Example**: 100,000 emails/month ≈ $10 + $0.02 + $0.10 = **$10.12/month**

Compare to Resend: 100,000 emails = $100/month (10x more expensive)

## Warm-up Schedule

The Lambda enforces a gradual warm-up to build sender reputation:

| Week | Daily Limit |
|------|-------------|
| 1 | 50 → 600 |
| 2 | 700 → 1,600 |
| 3 | 1,800 → 6,000 |
| 4 | 8,000 → 50,000+ |

After 28 days, no daily limits apply.

## Monitoring

### CloudWatch Metrics

The Lambda logs structured JSON with tracing. Key metrics:

- `email_sent`: Count of successful sends
- `email_bounced`: Bounce count
- `email_complained`: Complaint count
- `suppression_blocked`: Emails blocked due to suppression

### Health Check

```bash
curl https://API_ID.execute-api.REGION.amazonaws.com/health
```

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "warm_up": {
    "is_complete": false,
    "progress_percent": 25.0,
    "daily_limit": 1600
  },
  "health_score": 98
}
```

## Local Testing

### Without AWS (Unit Tests)

```bash
# Run all unit tests
cargo test -p email-proxy

# Run with verbose output
cargo test -p email-proxy -- --nocapture
```

### With cargo-lambda (Local Lambda Simulation)

```bash
# Install cargo-lambda
cargo install cargo-lambda

# Run locally (simulates Lambda environment)
cargo lambda watch -p email-proxy

# In another terminal, test the endpoints:
curl -X POST http://localhost:9000/send \
  -H "Content-Type: application/json" \
  -d '{"from":"test@example.com","to":["recipient@example.com"],"subject":"Test","html":"<p>Hello</p>"}'

# Note: This won't actually send emails without AWS credentials
```

### With LocalStack (Full AWS Simulation)

```bash
# Start LocalStack
docker run --rm -p 4566:4566 localstack/localstack

# Configure AWS CLI for LocalStack
export AWS_ENDPOINT_URL=http://localhost:4566
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test

# Create SES identity
aws --endpoint-url=http://localhost:4566 ses verify-email-identity \
  --email-address noreply@getsignatures.org

# Run Lambda with LocalStack
AWS_ENDPOINT_URL=http://localhost:4566 cargo lambda watch -p email-proxy
```

## Known Gotchas & Limitations

### 1. In-Memory Suppression List (CRITICAL)

**Issue**: The current suppression list uses an in-memory `HashMap`. This is **lost on every Lambda cold start**.

**Impact**:
- Bounced emails may not be suppressed after cold start
- Could hurt sender reputation if bounced addresses are emailed again

**Production Fix Required**: Store suppression list in DynamoDB:

```rust
// TODO: Replace HashMap with DynamoDB table
// Table: email-proxy-suppressions
// PK: email (lowercased)
// Attributes: reason, added_at, bounce_type, etc.
```

### 2. Cold Start with AWS SDK

**Issue**: First invocation includes AWS SDK initialization (~100-150ms).

**Mitigation**:
- Already using `OnceCell` to initialize client once
- ARM64 architecture reduces cold start
- Consider provisioned concurrency for latency-critical paths

### 3. Warm-up State Not Persisted

**Issue**: Daily email count resets on cold start.

**Impact**: Could exceed warm-up limits if Lambda scales horizontally.

**Production Fix**: Store daily count in DynamoDB or use CloudWatch metric.

### 4. SNS Subscription Confirmation

**Issue**: SNS sends a SubscriptionConfirmation request that must be acknowledged.

**Current Behavior**: Logs the subscribe URL but doesn't auto-confirm.

**Production Fix**: Fetch the subscribe URL to confirm subscription.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `FROM_DOMAIN` | Verified SES domain | `getsignatures.org` |
| `DEFAULT_FROM` | Default sender address | `GetSignatures <noreply@getsignatures.org>` |
| `SES_CONFIGURATION_SET` | SES configuration set name | `docsign-transactional` |
| `WARM_UP_ENABLED` | Enable warm-up limits | `true` |
| `RATE_LIMIT_PER_SECOND` | Max emails per second | `14` |
| `RUST_LOG` | Log level | `email_proxy=info` |
| `RUST_BACKTRACE` | Enable backtraces | `1` (recommended) |

## Development

```bash
# Run tests
cargo test -p email-proxy

# Check compilation
cargo check -p email-proxy

# Format
cargo fmt -p email-proxy
```

## Sources

- [AWS Lambda Rust GA Announcement](https://aws.amazon.com/blogs/aws/aws-weekly-roundup-aws-lambda-load-balancers-amazon-dcv-amazon-linux-2023-and-more-november-17-2025/)
- [AWS Lambda Rust Logging](https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html)
- [Cargo Lambda Documentation](https://www.cargo-lambda.info/)
- [ARM64 Performance Benchmarks](https://www.techradar.com/pro/arm64-dominates-aws-lambda-in-2025-rust-4-5x-faster-than-x86-costs-30-less-across-all-workloads)
- [AWS SDK Rust Best Practices](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/best-practices.html)
