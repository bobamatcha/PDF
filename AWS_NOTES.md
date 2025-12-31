# AWS Setup Guide

Step-by-step guide for deploying AWS Lambda functions with SES email sending.

## Prerequisites

- AWS Account
- Domain you control (for email sending)
- Homebrew installed (macOS)

---

## Part 1: AWS CLI Setup

### Install AWS CLI

```bash
brew install awscli
```

### Create Access Keys

1. Go to AWS Console → Click your name (top right) → **Security credentials**
2. Scroll to **Access keys** → **Create access key**
3. Select **Command Line Interface (CLI)**
4. Check the acknowledgment box → **Next** → **Create access key**
5. **Copy both keys immediately** (secret is only shown once)

### Configure AWS CLI

```bash
aws configure
```

Enter when prompted:
- **AWS Access Key ID**: (paste your access key)
- **AWS Secret Access Key**: (paste your secret key)
- **Default region name**: `us-east-2` (or your preferred region)
- **Default output format**: `json`

Credentials are saved to `~/.aws/credentials`.

---

## Part 2: AWS SES Setup (Email Sending)

### Step 1: Add Domain Identity

1. Go to **AWS Console** → Search **SES** → **Amazon Simple Email Service**
2. Left sidebar → **Configuration: Identities** → **Create identity**
3. Select **Domain** → Enter your domain (e.g., `getsignatures.org`)
4. Check **Use a custom MAIL FROM domain**
   - Enter subdomain: `mail` (creates `mail.yourdomain.com`)
   - Behavior on MX failure: **Use default MAIL FROM domain**
5. Expand **Advanced DKIM settings**
   - Select **Easy DKIM**
   - DKIM signing key length: **RSA_2048_BIT**
   - **UNCHECK** "Publish DNS records to Route53" (if using external DNS like Cloudflare)
   - DKIM signatures: **Enabled**
6. Click **Create identity**

### Step 2: Add DNS Records

After creating the identity, AWS shows DNS records to add. In your DNS provider (e.g., Cloudflare):

#### DKIM Records (3 CNAMEs)
| Type | Name | Target | Proxy |
|------|------|--------|-------|
| CNAME | `abc123._domainkey` | `abc123.dkim.amazonses.com` | DNS only (off) |
| CNAME | `def456._domainkey` | `def456.dkim.amazonses.com` | DNS only (off) |
| CNAME | `ghi789._domainkey` | `ghi789.dkim.amazonses.com` | DNS only (off) |

*(Use actual values from AWS)*

#### MAIL FROM Records
| Type | Name | Value | Priority |
|------|------|-------|----------|
| MX | `mail` | `feedback-smtp.us-east-2.amazonses.com` | 10 |
| TXT | `mail` | `v=spf1 include:amazonses.com ~all` | - |

**Note**: For MX records, the priority (10) goes in a separate field, NOT in the mail server value.

#### DMARC Record
| Type | Name | Value |
|------|------|-------|
| TXT | `_dmarc` | `v=DMARC1; p=none;` |

**Important**: All email-related DNS records should have proxy **OFF** (DNS only / gray cloud in Cloudflare).

### Step 3: Request Production Access

By default, SES is in "sandbox mode" - can only send to verified emails.

1. In SES → **Account dashboard** → **Request production access**
2. Fill out:
   - **Mail type**: Transactional
   - **Website URL**: Your website
   - **Acknowledge**: Check the box
3. Submit and wait for approval (usually 24 hours)

If AWS asks for more details, respond with:
- Use case (transactional emails like signing requests)
- Expected volume
- How you handle bounces/complaints (suppression list)
- Recipient list management (user-initiated only)

---

## Part 3: Lambda IAM Role

### Create IAM Role

1. Go to **IAM** → **Roles** → **Create role**
2. Select **AWS service** → **Lambda** → **Next**
3. Search and attach these policies:
   - `AWSLambdaBasicExecutionRole` (for CloudWatch logs)
   - `AmazonSESFullAccess` (for sending emails)
4. Click **Next**
5. **Role name**: `email-proxy-lambda-role` (or your preferred name)
6. **Description**: Optional
7. Skip permissions boundary (optional, for enterprise)
8. Click **Create role**
9. **Copy the Role ARN** (e.g., `arn:aws:iam::123456789:role/email-proxy-lambda-role`)

### IAM Policy Notes

| Policy | Purpose |
|--------|---------|
| `AWSLambdaBasicExecutionRole` | Write logs to CloudWatch |
| `AWSLambdaBasicDurableExecutionRolePolicy` | For Step Functions workflows (not needed for simple Lambdas) |
| `AmazonSESFullAccess` | Send emails via SES |

---

## Part 4: Deploy Lambda with cargo-lambda

### Install cargo-lambda

```bash
cargo install cargo-lambda
```

### Build Lambda

```bash
cd crates/email-proxy  # or your lambda crate
cargo lambda build --release --arm64
```

### Deploy Lambda

```bash
cargo lambda deploy email-proxy --iam-role arn:aws:iam::YOUR_ACCOUNT_ID:role/email-proxy-lambda-role
```

### Create Function URL (Public HTTP Endpoint)

```bash
aws lambda create-function-url-config \
  --function-name email-proxy \
  --auth-type NONE \
  --cors '{"AllowOrigins":["*"],"AllowMethods":["POST","GET"],"AllowHeaders":["content-type"]}'
```

### Add Public Access Permission

```bash
aws lambda add-permission \
  --function-name email-proxy \
  --statement-id public-access \
  --action lambda:InvokeFunctionUrl \
  --principal "*" \
  --function-url-auth-type NONE
```

### Set Environment Variables

```bash
aws lambda update-function-configuration \
  --function-name email-proxy \
  --environment 'Variables={FROM_EMAIL=noreply@mail.yourdomain.com,FROM_NAME=YourApp}'
```

### Test Health Endpoint

```bash
curl https://YOUR_FUNCTION_URL/health
```

Should return: `{"status":"ok"}`

### Test Email Sending (Sandbox Mode)

In sandbox mode, you can only send to verified emails (your AWS account email is auto-verified):

```bash
curl -X POST https://YOUR_FUNCTION_URL/send \
  -H "Content-Type: application/json" \
  -d '{
    "to": "your-verified-email@example.com",
    "subject": "Test Email",
    "html": "<h1>It works!</h1>"
  }'
```

---

## Part 5: Useful Commands

### View Lambda Logs

```bash
aws logs tail /aws/lambda/email-proxy --follow
```

### Update Lambda Code

```bash
cargo lambda build --release --arm64
cargo lambda deploy email-proxy
```

### Delete Lambda

```bash
aws lambda delete-function --function-name email-proxy
```

### List Function URLs

```bash
aws lambda list-function-url-configs --function-name email-proxy
```

### Check SES Sending Stats

```bash
aws ses get-send-statistics
```

### Check SES Identity Verification Status

```bash
aws ses get-identity-verification-attributes --identities yourdomain.com
```

---

## Troubleshooting

### "Forbidden" on Function URL
Add public access permission (see Part 4).

### "Credential provider not enabled"
Run `aws configure` with valid access keys.

### "Email address is not verified"
In sandbox mode, both sender and recipient must be verified. Request production access to send to anyone.

### DNS records not verifying
- Wait 5-72 hours for DNS propagation
- Ensure proxy is OFF for email records (Cloudflare)
- Check for typos in record values

### Lambda timeout
Default is 3 seconds. Increase if needed:
```bash
aws lambda update-function-configuration \
  --function-name email-proxy \
  --timeout 30
```

---

## Security Best Practices

1. **Never commit AWS credentials** - Use `aws configure` or environment variables
2. **Rotate access keys regularly** - Delete and recreate every 90 days
3. **Use IAM users, not root** - Create IAM user with minimal permissions for production
4. **Enable MFA** - On your AWS root account
5. **Use least privilege** - Only grant permissions Lambda actually needs

---

## Cost Notes

- **Lambda**: First 1M requests/month free, then $0.20 per 1M
- **SES**: First 62,000 emails/month free (if sent from Lambda), then $0.10 per 1,000
- **CloudWatch Logs**: First 5GB/month free

For most MVPs, you'll stay in the free tier.

---

## GetSignatures Deployment Details

### Email Proxy Lambda

| Resource | Value |
|----------|-------|
| Function Name | `email-proxy` |
| Region | `us-east-2` |
| ARN | `arn:aws:lambda:us-east-2:085096851463:function:email-proxy` |
| Function URL | `https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws` |
| IAM Role | `arn:aws:iam::085096851463:role/email-proxy-lambda-role` |

### SES Configuration

| Resource | Value |
|----------|-------|
| Domain Identity | `getsignatures.org` (verified) |
| MAIL FROM Domain | `mail.getsignatures.org` |
| From Address | `noreply@getsignatures.org` |
| Region | `us-east-2` |

### API Endpoints

```bash
# Health check
curl https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws/health

# Send email
curl -X POST https://5wbbpgjw7acyu4sgjqksmsqtvq0zajks.lambda-url.us-east-2.on.aws/send \
  -H "Content-Type: application/json" \
  -d '{
    "from": "GetSignatures <noreply@getsignatures.org>",
    "to": ["recipient@example.com"],
    "subject": "Your Subject",
    "html": "<h1>HTML content</h1>"
  }'
```

### Production Access Status

- **Status**: Pending approval (requested 2025-12-31)
- **Sandbox limitation**: Can only send to verified email addresses
- **After approval**: Can send to any email address

### Verified Email Addresses (Sandbox Mode)

To verify a new recipient for testing:
```bash
aws ses verify-email-identity --email-address recipient@example.com
```

Currently verified:
- `bobamatchasolutions@gmail.com`
