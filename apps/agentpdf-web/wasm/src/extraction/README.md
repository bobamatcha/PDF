# PDF Text Extraction Module

Smart PDF text extraction with intelligent routing between multiple backends.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     ExtractionRouter                            │
│  (Auto strategy - default)                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. quick_analyze() - O(n) byte scan, ~5μs                      │
│     ├── UseLegacy      → Legacy backend (fast path)             │
│     ├── ProbablyLegacy → Legacy with validation                 │
│     ├── NeedsFullAnalysis → parse_and_analyze()                 │
│     └── Invalid        → Error                                  │
│                                                                 │
│  2. parse_and_analyze() - Parse once, share Document            │
│     ├── Easy/Medium    → Native (reuses Document)               │
│     ├── Hard/VeryHard  → Browser (pdf.js)                       │
│     └── RequiresOcr    → Error (not implemented)                │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                        Backends                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │
│  │  Legacy  │  │  Native  │  │ Browser  │                       │
│  │pdf-extract│  │  lopdf   │  │  pdf.js  │                       │
│  │ fastest  │  │ enhanced │  │ fallback │                       │
│  └──────────┘  └──────────┘  └──────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

## Routing Logic

| PDF Characteristics | Quick Analysis Result | Backend | Reason |
|---------------------|----------------------|---------|--------|
| < 50KB | `UseLegacy` | Legacy | Fastest, no analysis needed |
| 50-100KB, no Identity-H | `UseLegacy` | Legacy | Simple encoding |
| > 100KB, no Identity-H | `ProbablyLegacy` | Legacy + validation | May need fallback |
| Any size with Identity-H | `NeedsFullAnalysis` | Native or Browser | Complex encoding |
| Identity-H + ToUnicode | - | Native | Can decode properly |
| Identity-H, no ToUnicode | - | Browser | Only pdf.js works |

## Performance Optimization

The routing uses a tiered analysis approach to minimize overhead:

### Before Optimization
```
Auto Strategy:
  1. PdfAnalysis::analyze() - Full parse (~35ms)
  2. Route to backend
  3. Backend parses again (~35ms)
  Total overhead: ~70ms for complex PDFs
```

### After Optimization
```
Auto Strategy:
  1. quick_analyze() - Byte scan (~5μs)
  2. If simple: Legacy directly (0ms overhead)
  3. If complex: parse_and_analyze() once, share Document
  Total overhead: ~5μs for simple PDFs, ~35ms for complex (single parse)
```

### Benchmark Results

| Scenario | Before | After | Speedup |
|----------|--------|-------|---------|
| Routing decision (simple PDF) | ~35ms | ~5μs | **7,600x** |
| Complex PDF total parsing | 2 parses | 1 parse | **2x** |

## Running Benchmarks

### Quick Analysis vs Full Analysis
```bash
cargo test -p agentpdf-wasm benchmark_quick_vs_full_analysis -- --nocapture
```

### Strategy Comparison (Legacy vs Auto)
```bash
cargo test -p agentpdf-wasm benchmark_legacy_vs_auto -- --nocapture
```

### Multi-run Benchmark
```bash
cargo test -p agentpdf-wasm benchmark_multiple_runs -- --nocapture
```

### All Benchmarks
```bash
cargo test -p agentpdf-wasm benchmark -- --nocapture
```

## Usage

### JavaScript (WASM)

```javascript
// Auto routing (recommended) - picks best backend automatically
const text = await extract_text_hybrid(pdfData);

// Force specific strategy
const text = await extract_text_with_strategy(pdfData, "legacy");
const text = await extract_text_with_strategy(pdfData, "auto");
const text = await extract_text_with_strategy(pdfData, "native");
const text = await extract_text_with_strategy(pdfData, "browser");

// Get extraction metadata
const result = await extract_with_metadata(pdfData, "auto");
console.log(result.backend_used);      // "legacy", "native", or "browser"
console.log(result.extraction_time_ms); // Time taken
console.log(result.fallback_occurred);  // Whether fallback was used
```

### Rust

```rust
use agentpdf_wasm::extraction::{
    ExtractionRouter, ExtractionConfig, ExtractionStrategy,
    quick_analyze, QuickAnalysis,
};

// Quick analysis for routing decisions
let quick = quick_analyze(&pdf_bytes);
match quick {
    QuickAnalysis::UseLegacy => { /* Use legacy directly */ }
    QuickAnalysis::NeedsFullAnalysis => { /* Parse and analyze */ }
    // ...
}

// Full extraction with router
let config = ExtractionConfig::default(); // Uses Auto strategy
let router = ExtractionRouter::new(config);
let result = router.extract(&pdf_bytes).await?;
```

## Module Structure

```
extraction/
├── mod.rs          # Public API exports
├── router.rs       # ExtractionRouter with strategy dispatch
├── analyzer.rs     # PdfAnalysis, quick_analyze
├── types.rs        # Common types (PageContent, ValidationResult, etc.)
├── legacy.rs       # LegacyExtractor (pdf-extract)
├── native.rs       # NativeExtractor (enhanced lopdf)
├── browser.rs      # BrowserExtractor (pdf.js bridge)
└── benchmark.rs    # BenchmarkRunner for accuracy testing
```

## CI/CD

Performance regression tests run on every PR. Regressions >5% will fail the build.

### Pre-commit Hook

Install the pre-commit hook to catch regressions before pushing:

```bash
make hooks-install
```

The hook will:
- Run tests when extraction code changes
- Check for benchmark regressions >5%
- Block commits that introduce performance regressions

To skip (not recommended): `git commit --no-verify`

### GitHub Actions

See `.github/workflows/benchmark.yml` for CI configuration.

The workflow:
1. Runs on PRs touching extraction code
2. Compares quick_analyze performance against baseline
3. Posts results as PR comment
4. Fails if regression >5%
