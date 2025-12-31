# Security Documentation for DocSign

This document outlines the security measures implemented in the DocSign application, known limitations, and the responsible disclosure process.

## Table of Contents

1. [Overview](#overview)
2. [Security Measures Implemented](#security-measures-implemented)
3. [Known Limitations](#known-limitations)
4. [Security Findings from Audit](#security-findings-from-audit)
5. [Responsible Disclosure](#responsible-disclosure)

---

## Overview

DocSign is a local-first document signing application with both web (docsign-web) and desktop (docsign-tauri) variants. The application handles sensitive data including PDF documents and digital signatures.

### Security Principles

1. **Local-First Architecture**: Documents and signatures are processed locally, minimizing data exposure
2. **Zero-Knowledge Design**: Server sync is optional; core functionality works offline
3. **Defense in Depth**: Multiple layers of validation and sanitization

---

## Security Measures Implemented

### 1. Input Validation

#### Session Parameters (session.ts)

- **Session ID**: Must be non-empty and at least 3 characters
- **Recipient ID**: Must be non-empty
- **Signing Key**: Must be non-empty and at least 3 characters
- All parameters are validated before processing

```typescript
// Validation rules enforced:
// - Session ID: length >= 3
// - Recipient ID: length >= 1
// - Signing Key: length >= 3
```

#### Filename Sanitization (file_dialogs.rs)

Path traversal attacks are prevented through filename sanitization:

```rust
// Dangerous characters replaced or removed:
// - Path separators: / \ :
// - Shell metacharacters: * ? " < > |
// - Control characters: 0x00-0x1F, 0x7F
// - Leading/trailing dots trimmed
// - Maximum length: 200 characters
```

#### Printer Name Validation (print.rs)

Command injection is prevented through strict validation:

```rust
// Rejected characters that could enable command injection:
// ' " ; & | ` $ \ \n \r
// Maximum length: 256 characters
```

### 2. XSS Prevention

#### HTML Escaping (error-ui.ts)

All user-provided content displayed in the UI is escaped:

```typescript
function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;  // Safe: uses textContent
  return div.innerHTML;    // Returns escaped HTML
}
```

**Protected locations:**
- Error modal titles and messages
- Toast notifications
- Confirmation dialogs

### 3. Cryptographic Security

#### Secure Random Generation

Session IDs use cryptographically secure random generation:

```typescript
// Uses crypto.randomUUID() - NOT Math.random()
const sessionId = crypto.randomUUID();
```

#### Web Crypto API Usage

For test scenarios requiring encryption, the Web Crypto API is used:

```typescript
const iv = crypto.getRandomValues(new Uint8Array(12));
```

### 4. File Handling Security

#### File Size Limits

Maximum file size enforced: **100 MB**

```rust
pub const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;
```

#### PDF Extension Enforcement

All saved files are forced to have `.pdf` extension to prevent disguised executables.

#### Temporary File Management

- Temporary files are created with `.pdf` suffix
- Cleanup is scheduled after use (60-second delay for print operations)
- Uses `tempfile` crate for secure temp file creation

### 5. Tauri Desktop Security

#### File System Scope

```json
{
  "fs": {
    "scope": {
      "allow": ["$DOCUMENT/**", "$HOME/**", "$DOWNLOAD/**"],
      "deny": ["$HOME/.ssh/**", "$HOME/.gnupg/**"]
    }
  }
}
```

**Protected directories:**
- `$HOME/.ssh/**` - SSH keys
- `$HOME/.gnupg/**` - GPG keys

#### Plugin Permissions

| Plugin | Permission | Purpose |
|--------|------------|---------|
| shell | open | Opening URLs in default browser |
| dialog | open, save | Native file dialogs |
| fs | scoped | File read/write with restrictions |
| updater | enabled | Auto-update functionality |

---

## Known Limitations

### 1. CSP Configuration

**Status**: CSP is currently set to `null` in Tauri configuration.

**Risk**: Without CSP, the application has no browser-enforced restrictions on resource loading.

**Recommendation**: Configure CSP for production:

```json
{
  "security": {
    "csp": "default-src 'self'; script-src 'self' https://cdnjs.cloudflare.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob:; connect-src 'self' https://releases.getsignatures.org"
  }
}
```

### 2. Broad File System Access

**Status**: `$HOME/**` allows access to entire home directory.

**Risk**: If the application is compromised, it could access any file in the user's home directory.

**Recommendation**: Narrow scope to specific directories:
- `$HOME/Documents/**`
- `$HOME/Desktop/**`
- `$HOME/Downloads/**`

### 3. LocalStorage Data at Rest

**Status**: Session data including signing keys is stored in IndexedDB without encryption.

**Risk**: Physical access to the device could expose signing keys.

**Recommendation**: Consider encrypting sensitive data at rest using Web Crypto API.

### 4. Updater Public Key

**Status**: Placeholder value `REPLACE_WITH_YOUR_PUBLIC_KEY` in tauri.conf.json.

**Risk**: Auto-updates cannot be verified without a valid public key.

**Action Required**: Replace with actual public key before production deployment.

---

## Security Findings from Audit

### High Priority

| Finding | Location | Status |
|---------|----------|--------|
| CSP disabled | tauri.conf.json | Open |
| Updater key placeholder | tauri.conf.json | Open |

### Medium Priority

| Finding | Location | Status |
|---------|----------|--------|
| Broad FS scope ($HOME/**) | tauri.conf.json | Open |
| Unencrypted IndexedDB storage | local-session-manager.ts | Open |

### Low Priority

| Finding | Location | Status |
|---------|----------|--------|
| Whitespace-only session IDs accepted | session.ts | Documented |

### Mitigated

| Finding | Mitigation |
|---------|------------|
| Command injection in printer names | Strict character validation |
| Path traversal in filenames | Comprehensive sanitization |
| XSS in error messages | HTML escaping via textContent |
| Insecure random for sessions | Uses crypto.randomUUID() |

---

## Responsible Disclosure

### Reporting Security Issues

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT** create a public GitHub issue
2. Email security findings to: **security@getsignatures.org**
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### Response Timeline

| Phase | Timeline |
|-------|----------|
| Acknowledgment | Within 48 hours |
| Initial Assessment | Within 7 days |
| Resolution Target | Within 90 days |
| Public Disclosure | After fix is released |

### Bug Bounty

Currently, there is no formal bug bounty program. However, we appreciate and acknowledge security researchers who report valid vulnerabilities responsibly.

### Hall of Fame

Security researchers who have contributed to improving DocSign's security will be acknowledged here (with permission).

---

## Security Testing

### Running Security Tests

```bash
cd apps/docsign-web
npm test -- src/ts/__tests__/security.test.ts
```

### Test Coverage

The security test suite covers:
- XSS prevention (HTML escaping)
- Input sanitization (session params, filenames)
- Command injection prevention (printer names)
- Cryptographic security (random generation)
- File size validation
- Path traversal prevention

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-30 | 1.0 | Initial security documentation |

---

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Tauri Security](https://tauri.app/v1/guides/security/)
- [Web Crypto API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Crypto_API)
- [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
