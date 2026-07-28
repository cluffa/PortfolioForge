## Project

PortfolioForge — privacy-first PDF Portfolio creator. Rust CLI + WASM web app. All processing local, no uploads, no accounts.

## Build & Run

```bash
# CLI
cargo build -p pfcli
cargo run -p pfcli -- create output.pdf input1.pdf input2.png

# Web app
./serve.sh        # builds WASM + starts local server on :8080

# Tests
cargo test --workspace
```

## Architecture

```
crates/
  portfolio-core/     PDF Portfolio CRUD (lopdf 0.44)
  portfolio-wasm/     wasm-bindgen wrapper for browser
  image-converter/    PNG/JPG/GIF/WebP → PDF (lopdf + image 0.25)
  docx-converter/     DOCX → text PDF (docx-rust)
  pfcli/              CLI (clap)

apps/web/             Vanilla HTML/CSS/JS — drag/drop → WASM
  index.html, style.css, app.js
  public/wasm/        wasm-pack output (gitignored, CI builds it)

samples/
  Portfolio1.pdf      Real Acrobat portfolio (9 files, for tests)
  Portfolio2.pdf      Acrobat blank template
  acrobat_portfolio_baseline.pdf  Acrobat-created reference portfolio
```

## Key Technical Details

- **lopdf 0.44**: Must use `get_deref` for catalog lookups (not plain `get` — doesn't follow references)
- **Writer**: Uses Portfolio2.pdf template (preserves Adobe cover page, Schema, Sort, Folders). File streams compressed via `stream.compress()`.
- **Reader**: Must decompress streams via `stream.decompressed_content()` when extracting.
- **WASM**: `getrandom` 0.4 needs `wasm_js` feature. Portfolio merges (adding a .pdf portfolio expands its files).
- **CI**: `.github/workflows/deploy.yml` builds WASM + deploys to GitHub Pages. Settings → Pages → Source: GitHub Actions.

## Commands

```
pf create output.pdf files...    Create portfolio (auto-converts images/docx)
pf list portfolio.pdf            List embedded files
pf extract portfolio.pdf dir/    Extract all files
pf info portfolio.pdf            Show metadata
pf validate portfolio.pdf        Check structure
pf add portfolio.pdf files...    Add files to existing
pf remove portfolio.pdf name     Remove file
pf replace portfolio.pdf old new Replace file
pf rename portfolio.pdf old new  Rename file
pf reorder portfolio.pdf a b c   Reorder files
```
