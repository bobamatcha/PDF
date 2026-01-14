# ELI5: Testing DocSign Locally Before Deployment

## Quick Start (TL;DR)

```bash
# Terminal 1: Start the backend API
cd apps/docsign-api
cargo run

# Terminal 2: Start the web app
cd apps/docsign-web
npm run dev

# Open in browser: http://localhost:8080
```

---

## What Is This?

**DocSign** is a website where people can sign documents electronically (like DocuSign, but for elderly users).

There are **3 parts** that work together:

1. **Backend API** (`docsign-api`) - The "brain" that stores signing sessions
2. **Web App** (`docsign-web`) - The website users see in their browser
3. **Desktop App** (`docsign-tauri`) - Optional native app for Mac/Windows/Linux

---

## Step-by-Step Local Testing

### Step 1: Open Your Terminal

On Mac: Press `Cmd + Space`, type "Terminal", press Enter

### Step 2: Go to the Project Folder

```bash
cd /Users/amar/AG1337v2/BobaMatchSolutions/PDF/m3-getsigsmvp
```

### Step 3: Start the Backend API

Think of this as starting up the "server" that handles data:

```bash
cd apps/docsign-api
cargo run
```

You should see:
```
Starting DocSign API on http://0.0.0.0:3001
```

**Leave this terminal running!** (Don't close it)

### Step 4: Open a NEW Terminal Window

Press `Cmd + N` to open a new terminal, then:

```bash
cd /Users/amar/AG1337v2/BobaMatchSolutions/PDF/m3-getsigsmvp/apps/docsign-web
npm run dev
```

You should see something like:
```
Server running at http://localhost:8080
```

### Step 5: Open the Website in Your Browser

1. Open Safari, Chrome, or Firefox
2. Go to: `http://localhost:8080`
3. You should see the DocSign website!

---

## Testing the Signing Flow

Since the app is "local-first", it works even without a signing link from the server. But to test the full flow:

### Option A: Test Offline Mode

1. Open `http://localhost:8080/sign.html`
2. The page will show an error (no session) - this is expected!
3. But you can still test the signature modal by pressing F12 (Developer Tools) and running:
   ```javascript
   document.getElementById('signature-modal').classList.remove('hidden');
   ```

### Option B: Create a Test Session via API

```bash
# In a new terminal:
curl -X POST http://localhost:3001/api/session \
  -H "Content-Type: application/json" \
  -d '{
    "document_name": "Test Contract",
    "pdf_base64": "JVBERi0xLjQKMSAwIG9iago8PAovVHlwZSAvQ2F0YWxvZwovUGFnZXMgMiAwIFIKPj4KZW5kb2JqCjIgMCBvYmoKPDwKL1R5cGUgL1BhZ2VzCi9LaWRzIFszIDAgUl0KL0NvdW50IDEKL01lZGlhQm94IFswIDAgNjEyIDc5Ml0KPj4KZW5kb2JqCjMgMCBvYmoKPDwKL1R5cGUgL1BhZ2UKL1BhcmVudCAyIDAgUgo+PgplbmRvYmoKeHJlZgowIDQKMDAwMDAwMDAwMCA2NTUzNSBmIAowMDAwMDAwMDA5IDAwMDAwIG4gCjAwMDAwMDAwNTggMDAwMDAgbiAKMDAwMDAwMDE0OCAwMDAwMCBuIAp0cmFpbGVyCjw8Ci9TaXplIDQKL1Jvb3QgMSAwIFIKPj4Kc3RhcnR4cmVmCjE5NQolJUVPRgo=",
    "recipients": [{"id": "r1", "name": "John Doe", "email": "john@example.com", "role": "signer", "status": "pending"}],
    "fields": [{"id": "f1", "field_type": "signature", "page": 1, "x": 100, "y": 100, "width": 200, "height": 50, "recipient_id": "r1", "required": true}]
  }'
```

This will return a session ID that you can use.

---

## Testing the Signature Modal

The most important part to test is the signature capture:

1. Open `http://localhost:8080/sign.html`
2. Even with the error, you can test the UI components
3. Check that:
   - [ ] Buttons are at least 60px tall (good for elderly users)
   - [ ] Text is at least 18px (easy to read)
   - [ ] The Undo button works (↶ Undo)
   - [ ] Ctrl+Z keyboard shortcut works
   - [ ] Type tab is the default (easier than drawing)

---

## Testing the Desktop App (Optional)

If you want to test the native Mac/Windows/Linux app:

```bash
cd apps/docsign-tauri
npm install
cargo tauri dev
```

This opens a native window with the same web content.

---

## Common Issues & Fixes

### "Port already in use"

```bash
# Kill whatever is using port 3001
lsof -i :3001
kill -9 <PID>
```

### "npm not found"

You need Node.js installed:
```bash
brew install node
```

### "cargo not found"

You need Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Quick Test Checklist

Before deploying, verify:

- [ ] Backend starts without errors (`cargo run` in docsign-api)
- [ ] Web app starts without errors (`npm run dev` in docsign-web)
- [ ] Website loads in browser at http://localhost:8080
- [ ] All tests pass: `npm test -- --run` (399 tests)
- [ ] Rust tests pass: `cargo test -p docsign-api` (21 tests)
- [ ] Buttons are big (60px+)
- [ ] Text is readable (18px+)
- [ ] Undo button works
- [ ] Ctrl+Z works

---

## Next: Deployment

See `DEPLOYMENT.md` for how to put this on a real server.
