# PortfolioForge v0.1 — Design

Date: 2025-07-28

## Scope

Phase 0.5 feasibility + Phase 1 (Core Reader) + Phase 2 (Portfolio Writer) from the spec. A Rust CLI that can create, list, extract, and inspect PDF Portfolios. No document conversion yet.

## Architecture

```
crates/
├── portfolio-core/       # PDF Portfolio create/list/extract (depends: lopdf)
├── image-converter/      # Image → PDF page [stubbed]
├── docx-converter/       # DOCX → PDF pages [stubbed]
└── pfcli/                # CLI binary (depends: portfolio-core, clap)
```

### portfolio-core

Wraps `lopdf` with Portfolio-specific abstractions.

**Public API:**

```rust
// Detection
Portfolio::is_portfolio(data: &[u8]) -> bool

// Open/Read
Portfolio::open(data: &[u8]) -> Result<Portfolio>
portfolio.files() -> &[EmbeddedFile]
portfolio.metadata() -> PortfolioMetadata

// Create
Portfolio::create(template: &[u8]) -> PortfolioBuilder
PortfolioBuilder::add_file(name, data, mime) -> &mut Self
PortfolioBuilder::set_view(ViewMode) -> &mut Self
PortfolioBuilder::build() -> Result<Vec<u8>>

// Extract
portfolio.extract_all(output_dir: &Path) -> Result<Vec<PathBuf>>
```

**Types:**
- `EmbeddedFile { name: String, size: u64, mime_type: String, data: Vec<u8> }`
- `PortfolioMetadata { pdf_version: String, view_mode: Option<ViewMode>, file_count: usize }`
- `ViewMode { Details, Tile, Hidden }`

**Error type:** `PortfolioError` using `thiserror`, covering parse failures, invalid PDF, extraction errors, and creation failures.

### pfcli

Commands:

```
pf create <output> <files...>     Create a new PDF Portfolio
pf list <portfolio>               List embedded documents  
pf extract <portfolio> [dir]      Extract embedded documents
pf info <portfolio>               Show metadata
```

Uses `clap` derive API. `anyhow` for error display.

### How Portfolio creation works

1. Start from `Portfolio2.pdf` (blank PDF 1.7 template from Acrobat)
2. Add `/Collection` dictionary to catalog with `/View D`
3. For each file, add a `/Filespec` entry and `/EmbeddedFile` stream to the `/EmbeddedFiles` name tree
4. Write with `lopdf::Document::save()`, which handles cross-reference streams automatically

### How Portfolio reading works

1. `lopdf::Document::load()` parses the PDF
2. Check catalog for `/Collection` key → detection
3. Walk `/EmbeddedFiles` name tree to enumerate files
4. Extract each file stream's raw bytes

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| lopdf | 0.34 | PDF parsing, object manipulation, cross-reference streams |
| clap | 4.5 | CLI argument parsing (derive) |
| thiserror | 2 | Typed error types |
| anyhow | 1 | CLI error propagation |

## Testing

- Unit tests for `portfolio-core` using `minimal_portfolio.pdf` and `Portfolio1.pdf` as fixtures
- Integration test: `pf create → pf list → pf extract → diff` roundtrip
- Test that `Portfolio1.pdf` can be read and files extracted
- Test that a created portfolio opens correctly when inspected with `lopdf`

## What's deferred

- Image conversion (PNG/JPEG/TIFF/WebP → PDF)
- DOCX conversion
- Cover page generation
- Portfolio modification (add/remove/replace/reorder)
- WASM compilation
- Web UI
