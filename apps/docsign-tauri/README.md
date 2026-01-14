# docsign-tauri

GetSignatures desktop application built with Tauri 2.0.

## Overview

docsign-tauri wraps the docsign-web frontend in a native desktop application, adding:

- **Native file dialogs** - Open/save PDFs with system dialogs
- **Native printing** - Direct printer access (macOS, Windows, Linux)
- **System tray** - Hide-to-tray, quick access menu
- **Auto-updates** - Automatic update checking and installation
- **Offline-first** - No network required for signing

## Requirements

- Rust 1.70+
- Node.js 18+
- Platform-specific requirements:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools
  - **Linux**: `webkit2gtk-4.1`, `libappindicator3`

## Quick Start

```bash
# Install dependencies
npm install

# Sync frontend from docsign-web
npm run sync-frontend

# Start development
npm run dev

# Build for production
npm run build
```

## Available Scripts

| Script | Description |
|--------|-------------|
| `npm run dev` | Start Tauri dev server |
| `npm run build` | Build release application |
| `npm run build:debug` | Build with debug symbols |
| `npm run sync-frontend` | Sync www/ from docsign-web |
| `npm run typecheck` | Type-check TypeScript |
| `npm test` | Run vitest tests |

## Architecture

```
docsign-tauri/
├── package.json           # NPM scripts
├── src/                   # Frontend (synced from docsign-web/www)
│   ├── index.html
│   ├── sign.html
│   └── js/
└── src-tauri/             # Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json    # Tauri configuration
    ├── icons/             # App icons
    └── src/
        ├── main.rs        # Entry point
        ├── lib.rs         # App setup, plugins
        ├── tray.rs        # System tray
        └── commands/
            ├── mod.rs
            ├── file_dialogs.rs
            ├── print.rs
            └── updater.rs
```

## Build Configuration

### tauri.conf.json

```json
{
  "productName": "GetSignatures",
  "version": "0.1.0",
  "identifier": "org.getsignatures.app",
  "app": {
    "windows": [{
      "width": 1200,
      "height": 900,
      "minWidth": 900,
      "minHeight": 700,
      "title": "GetSignatures - Document Signing"
    }]
  }
}
```

### Plugins

| Plugin | Purpose |
|--------|---------|
| `tauri-plugin-dialog` | Native open/save dialogs |
| `tauri-plugin-fs` | File system access |
| `tauri-plugin-shell` | Open URLs in browser |
| `tauri-plugin-updater` | Auto-update support |

## Native Features

### File Dialogs

```typescript
// From frontend
const result = await invoke('open_pdf_file');
if (result) {
  // result is Uint8Array of PDF bytes
}

await invoke('save_signed_pdf', {
  pdfBytes: signedPdf,
  suggestedName: 'contract_signed.pdf'
});
```

### Printing

```typescript
// Print with system dialog
await invoke('print_pdf', { pdfBytes });

// Print to specific printer
const printers = await invoke('get_available_printers');
await invoke('print_to_printer', {
  pdfBytes,
  printerName: printers[0].name
});
```

### System Tray

The app minimizes to system tray on close:

- **Left-click**: Show/focus window
- **Right-click**: Context menu
  - Open GetSignatures
  - Recent Documents
  - Quit

### Auto-Updates

```typescript
// Check for updates
const update = await invoke('check_for_updates');
if (update) {
  console.log(`Update ${update.version} available`);
  await invoke('install_update');
}
```

## Development Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Install Tauri CLI

```bash
cargo install tauri-cli
```

### 3. Platform Dependencies

**macOS:**
```bash
xcode-select --install
```

**Ubuntu/Debian:**
```bash
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

**Windows:**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Select "Desktop development with C++"

### 4. Sync Frontend

```bash
npm run sync-frontend
```

This copies `../docsign-web/www/` to `./src/`.

### 5. Start Development

```bash
npm run dev
```

## Building for Release

### All Platforms

```bash
npm run build
```

### Platform-Specific

```bash
# macOS (creates .app and .dmg)
cargo tauri build --target aarch64-apple-darwin  # Apple Silicon
cargo tauri build --target x86_64-apple-darwin   # Intel

# Windows (creates .msi and .exe)
cargo tauri build --target x86_64-pc-windows-msvc

# Linux (creates .deb and .AppImage)
cargo tauri build --target x86_64-unknown-linux-gnu
```

### Output Locations

```
src-tauri/target/release/bundle/
├── macos/
│   ├── GetSignatures.app
│   └── GetSignatures.dmg
├── msi/
│   └── GetSignatures_x.x.x_x64_en-US.msi
├── nsis/
│   └── GetSignatures_x.x.x_x64-setup.exe
└── deb/
    └── get-signatures_x.x.x_amd64.deb
```

## Auto-Update Configuration

### Update Server

Configure in `tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://releases.getsignatures.org/updates/{{target}}/{{arch}}/{{current_version}}"
      ],
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

### Generate Keys

```bash
cargo tauri signer generate
```

This creates:
- Public key (for `tauri.conf.json`)
- Private key (for signing releases)

## Testing

### TypeScript Tests

```bash
npm test
```

### Rust Tests

```bash
cd src-tauri
cargo test

# Specific crate tests
cargo test -p docsign-tauri file_dialogs
cargo test -p docsign-tauri print
```

### Test Coverage

| Module | Tests |
|--------|-------|
| `file_dialogs.rs` | 17 property tests |
| `print.rs` | 19 property tests |
| TypeScript bindings | 38 property tests |
| **Total** | 105+ tests |

## Tauri Commands Reference

### File Dialogs

```rust
#[tauri::command]
async fn open_pdf_file() -> Result<Option<Vec<u8>>, String>

#[tauri::command]
async fn save_signed_pdf(pdf_bytes: Vec<u8>, suggested_name: String) -> Result<Option<String>, String>

#[tauri::command]
async fn open_multiple_pdfs() -> Result<Vec<(String, Vec<u8>)>, String>
```

### Printing

```rust
#[tauri::command]
async fn print_pdf(pdf_bytes: Vec<u8>) -> Result<bool, String>

#[tauri::command]
async fn get_available_printers() -> Result<Vec<PrinterInfo>, String>

#[tauri::command]
async fn print_to_printer(pdf_bytes: Vec<u8>, printer_name: String) -> Result<bool, String>
```

### Window Management

```rust
#[tauri::command]
fn show_main_window() -> Result<(), String>

#[tauri::command]
fn hide_to_tray() -> Result<(), String>
```

### Updates

```rust
#[tauri::command]
async fn check_for_updates() -> Result<Option<UpdateInfo>, String>

#[tauri::command]
async fn install_update() -> Result<(), String>

#[tauri::command]
fn get_current_version() -> String
```

## Related Documentation

- [docsign-web README](../docsign-web/README.md)
- [DOCSIGN_PLAN.md](/DOCSIGN_PLAN.md)
- [Tauri Documentation](https://v2.tauri.app/)
