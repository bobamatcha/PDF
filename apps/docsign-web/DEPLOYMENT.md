# ELI5: Deploying DocSign to a Real Server

## What Does "Deployment" Mean?

Right now, DocSign only works on YOUR computer. "Deployment" means putting it on the internet so ANYONE can use it from their browser.

Think of it like this:
- **Local** = A restaurant that only you can eat at (in your kitchen)
- **Deployed** = A real restaurant anyone can walk into

---

## The Two Parts to Deploy

### 1. Backend API (docsign-api)
- This is the "brain" that stores signing sessions
- Runs on a server (like a computer that's always on)
- URL will be something like: `https://api.getsignatures.org`

### 2. Frontend Web App (docsign-web)
- This is the website users see
- Just static files (HTML, CSS, JavaScript)
- URL will be something like: `https://getsignatures.org`

---

## Option A: Deploy to Fly.io (Recommended for Beginners)

Fly.io is like a "server rental" service. They handle all the hard parts.

### Step 1: Install Fly CLI

```bash
# On Mac
brew install flyctl

# Or using curl
curl -L https://fly.io/install.sh | sh
```

### Step 2: Create a Fly.io Account

```bash
flyctl auth signup
```

This opens a browser. Create your account (free tier available).

### Step 3: Deploy the Backend API

```bash
cd /Users/amar/AG1337v2/BobaMatchSolutions/PDF/m3-getsigsmvp/apps/docsign-api

# Create a new Fly app
flyctl launch --name docsign-api

# When asked:
# - Region: Choose one close to your users (e.g., "sjc" for San Francisco)
# - PostgreSQL: No (we use SQLite)
# - Redis: No

# Deploy!
flyctl deploy
```

Your API is now live at: `https://docsign-api.fly.dev`

### Step 4: Deploy the Frontend

For the frontend (static files), use Cloudflare Pages or Vercel:

#### Using Cloudflare Pages:

1. Go to https://pages.cloudflare.com
2. Sign up / Log in
3. Click "Create a project"
4. Connect your GitHub repository
5. Configure build settings:
   - Build command: `cd apps/docsign-web && npm run build`
   - Build output directory: `apps/docsign-web/www`
6. Click "Save and Deploy"

Your site is now live at: `https://your-project.pages.dev`

---

## Option B: Deploy to a VPS (More Control)

A VPS (Virtual Private Server) is like renting a computer in a data center.

### Popular VPS Providers:
- **DigitalOcean** - $6/month for basic droplet
- **Linode** - $5/month
- **Vultr** - $5/month
- **Hetzner** - $4/month (Europe)

### Step 1: Create a VPS

1. Sign up at DigitalOcean (or your choice)
2. Create a "Droplet" (their word for VPS)
   - Image: Ubuntu 22.04
   - Size: Basic, $6/month (1GB RAM)
   - Region: Closest to your users
3. Add your SSH key (or use password)

### Step 2: SSH Into Your Server

```bash
ssh root@YOUR_SERVER_IP
```

### Step 3: Install Dependencies

```bash
# Update system
apt update && apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt install -y nodejs

# Install Nginx (web server)
apt install -y nginx
```

### Step 4: Clone and Build

```bash
# Clone your repository
git clone YOUR_REPO_URL /var/www/docsign
cd /var/www/docsign

# Build the API
cd apps/docsign-api
cargo build --release

# Build the frontend
cd ../docsign-web
npm install
npm run build
```

### Step 5: Set Up the API as a Service

Create `/etc/systemd/system/docsign-api.service`:

```ini
[Unit]
Description=DocSign API
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/var/www/docsign/apps/docsign-api
ExecStart=/var/www/docsign/apps/docsign-api/target/release/docsign-api
Restart=always
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

Start the service:

```bash
systemctl daemon-reload
systemctl enable docsign-api
systemctl start docsign-api
```

### Step 6: Configure Nginx

Create `/etc/nginx/sites-available/docsign`:

```nginx
# API server
server {
    listen 80;
    server_name api.getsignatures.org;

    location / {
        proxy_pass http://127.0.0.1:3001;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}

# Frontend
server {
    listen 80;
    server_name getsignatures.org www.getsignatures.org;

    root /var/www/docsign/apps/docsign-web/www;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

Enable the site:

```bash
ln -s /etc/nginx/sites-available/docsign /etc/nginx/sites-enabled/
nginx -t  # Test config
systemctl reload nginx
```

### Step 7: Add HTTPS (Important!)

```bash
# Install Certbot
apt install -y certbot python3-certbot-nginx

# Get SSL certificates
certbot --nginx -d getsignatures.org -d www.getsignatures.org -d api.getsignatures.org
```

---

## Option C: Deploy with Docker (Most Portable)

### Step 1: Create Dockerfiles

**Backend Dockerfile** (`apps/docsign-api/Dockerfile`):

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/docsign-api /usr/local/bin/
EXPOSE 3001
CMD ["docsign-api"]
```

**Frontend Dockerfile** (`apps/docsign-web/Dockerfile`):

```dockerfile
FROM node:20 as builder
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/www /usr/share/nginx/html
EXPOSE 80
```

### Step 2: Build and Push Images

```bash
# Build images
docker build -t docsign-api ./apps/docsign-api
docker build -t docsign-web ./apps/docsign-web

# Push to Docker Hub (or other registry)
docker tag docsign-api YOUR_USERNAME/docsign-api
docker push YOUR_USERNAME/docsign-api
```

### Step 3: Deploy with Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'
services:
  api:
    image: YOUR_USERNAME/docsign-api
    ports:
      - "3001:3001"
    restart: always

  web:
    image: YOUR_USERNAME/docsign-web
    ports:
      - "80:80"
    restart: always
```

Run on your server:

```bash
docker-compose up -d
```

---

## Update the Frontend API URL

Before deploying the frontend, update the API URL in the code:

**File: `apps/docsign-web/src/ts/config.ts`** (or wherever config lives):

```typescript
// Change from localhost to your production API
const API_URL = 'https://api.getsignatures.org';
```

Or use environment variables:

```typescript
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';
```

---

## DNS Setup

Once deployed, point your domain to your server:

1. Log into your domain registrar (GoDaddy, Namecheap, Cloudflare, etc.)
2. Add these DNS records:

| Type | Name | Value |
|------|------|-------|
| A | @ | YOUR_SERVER_IP |
| A | www | YOUR_SERVER_IP |
| A | api | YOUR_SERVER_IP |

Wait 5-30 minutes for DNS to propagate.

---

## Deployment Checklist

Before going live:

- [ ] All tests pass locally (`npm test -- --run` and `cargo test -p docsign-api`)
- [ ] Frontend builds without errors (`npm run build`)
- [ ] Backend builds without errors (`cargo build --release`)
- [ ] API URL updated in frontend config
- [ ] HTTPS enabled (SSL certificate)
- [ ] DNS records configured
- [ ] Backend service starts on boot
- [ ] Tested signing flow end-to-end on production URL

---

## Quick Troubleshooting

### "502 Bad Gateway"
- The API isn't running. Check: `systemctl status docsign-api`

### "Connection refused"
- Firewall blocking port. Run: `ufw allow 80,443,3001`

### "SSL certificate error"
- Certbot failed. Re-run: `certbot --nginx`

### "Changes not showing up"
- Clear browser cache or use incognito mode
- Rebuild and redeploy the frontend

---

## Cost Estimate

| Option | Monthly Cost |
|--------|-------------|
| Fly.io (free tier) | $0 |
| Fly.io (basic) | ~$5 |
| Cloudflare Pages | $0 (free tier) |
| DigitalOcean VPS | $6 |
| Domain name | ~$12/year |

**Cheapest option**: Fly.io free tier + Cloudflare Pages = **$0/month**

---

## Need Help?

- Fly.io docs: https://fly.io/docs
- DigitalOcean tutorials: https://www.digitalocean.com/community/tutorials
- Cloudflare Pages docs: https://developers.cloudflare.com/pages
