# PortfolioForge

Privacy-first PDF Portfolio creator. No uploads, no accounts, all processing local.

**Create, view, and edit Adobe-compatible PDF Portfolios in your browser or terminal.**

[![Deploy](https://github.com/cluffa/PortfolioForge/actions/workflows/deploy.yml/badge.svg)](https://github.com/cluffa/PortfolioForge/actions/workflows/deploy.yml)

## Try it

**Web:** [cluffa.github.io/PortfolioForge](https://cluffa.github.io/PortfolioForge/)

**CLI:**
```bash
cargo run -p pfcli -- create portfolio.pdf document.pdf photo.png
cargo run -p pfcli -- list portfolio.pdf
cargo run -p pfcli -- extract portfolio.pdf output/
```

## Features

- Create PDF Portfolios from PDFs, images (PNG/JPG/GIF/WebP/TIFF/BMP), and DOCX
- Auto-converts images and DOCX to PDF on import
- Open and extract files from existing portfolios
- Edit: add, remove, replace, rename, reorder files
- Merge portfolios (add one portfolio into another)
- All processing in your browser via WebAssembly — nothing leaves your device

## CLI Commands

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

## Build

```bash
# CLI
cargo build -p pfcli

# Web app (requires wasm-pack)
./serve.sh
```
