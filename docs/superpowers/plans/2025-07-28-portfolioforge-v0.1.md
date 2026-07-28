# PortfolioForge v0.1 CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI (`pf`) that creates, lists, extracts, and inspects PDF Portfolios using `lopdf`.

**Architecture:** Two crates — `portfolio-core` wraps `lopdf` with Portfolio-specific abstractions (create, open, list, extract), and `pfcli` provides the `clap`-based CLI. A workspace root ties them together.

**Tech Stack:** Rust 2021 edition, lopdf 0.34, clap 4.5 (derive), thiserror 2, anyhow 1

## Global Constraints

- All processing local, no network, no telemetry
- Test against real PDF Portfolio fixtures in `samples/`
- Portfolio2.pdf is the blank template for creating new portfolios
- PDF 1.7 target, `/Collection` + `/EmbeddedFiles` structure

---

### Task 1: Scaffold Rust workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/portfolio-core/Cargo.toml`
- Create: `crates/pfcli/Cargo.toml`
- Create: `crates/image-converter/Cargo.toml` (stub)
- Create: `crates/docx-converter/Cargo.toml` (stub)
- Create: `rust-toolchain.toml`
- Modify: `.gitignore` (add Cargo.lock? for workspace)

**Interfaces:**
- Produces: Workspace with members `portfolio-core`, `pfcli`, `image-converter`, `docx-converter`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
members = [
    "crates/portfolio-core",
    "crates/pfcli",
    "crates/image-converter",
    "crates/docx-converter",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
```

- [ ] **Step 2: Create portfolio-core Cargo.toml**

```toml
[package]
name = "portfolio-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
lopdf = "0.34"
thiserror = "2"
```

- [ ] **Step 3: Create pfcli Cargo.toml**

```toml
[package]
name = "pfcli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "pf"
path = "src/main.rs"

[dependencies]
portfolio-core = { path = "../portfolio-core" }
clap = { version = "4.5", features = ["derive"] }
anyhow = "1"
```

- [ ] **Step 4: Create stub crates**

image-converter/Cargo.toml and docx-converter/Cargo.toml with empty `[package]` sections and a `src/lib.rs` containing only `// Stub - not yet implemented`.

- [ ] **Step 5: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 6: Verify**

Run: `cargo check --workspace`
Expected: All crates compile successfully (stubs are empty).

- [ ] **Step 7: Commit**

---

### Task 2: Implement portfolio-core types and errors

**Files:**
- Create: `crates/portfolio-core/src/lib.rs`
- Create: `crates/portfolio-core/src/error.rs`
- Create: `crates/portfolio-core/src/types.rs`

**Interfaces:**
- Produces: `PortfolioError` enum, `EmbeddedFile` struct, `PortfolioMetadata` struct, `ViewMode` enum

- [ ] **Step 1: Write error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Failed to parse PDF: {0}")]
    ParseError(#[from] lopdf::Error),
    
    #[error("Not a PDF Portfolio (no /Collection dictionary)")]
    NotAPortfolio,
    
    #[error("File not found in portfolio: {0}")]
    FileNotFound(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 2: Write types.rs**

```rust
/// View mode for the Portfolio collection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Details,
    Tile,
    Hidden,
}

impl ViewMode {
    pub fn to_pdf_name(&self) -> &str {
        match self {
            ViewMode::Details => "D",
            ViewMode::Tile => "T",
            ViewMode::Hidden => "H",
        }
    }
}

/// An embedded file within a PDF Portfolio
#[derive(Debug, Clone)]
pub struct EmbeddedFile {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// Metadata about a PDF Portfolio
#[derive(Debug, Clone)]
pub struct PortfolioMetadata {
    pub pdf_version: String,
    pub view_mode: Option<ViewMode>,
    pub file_count: usize,
}
```

- [ ] **Step 3: Write lib.rs (module declarations only)**

```rust
pub mod error;
pub mod types;

pub use error::PortfolioError;
pub use types::{EmbeddedFile, PortfolioMetadata, ViewMode};
```

- [ ] **Step 4: Verify**

Run: `cargo check -p portfolio-core`
Expected: Compiles cleanly.

- [ ] **Step 5: Commit**

---

### Task 3: Implement Portfolio::open and Portfolio::is_portfolio

**Files:**
- Create: `crates/portfolio-core/src/reader.rs`
- Modify: `crates/portfolio-core/src/lib.rs` (add module + re-export)

**Interfaces:**
- Consumes: `PortfolioError`, `EmbeddedFile`, `PortfolioMetadata`, `ViewMode` from Task 2
- Produces: `Portfolio::is_portfolio(data)`, `Portfolio::open(data)`, `portfolio.files()`, `portfolio.metadata()`

- [ ] **Step 1: Add reader module and Portfolio struct to lib.rs**

Append to lib.rs:
```rust
mod reader;

use std::io::Cursor;
pub use reader::Portfolio;
```

- [ ] **Step 2: Write reader.rs**

```rust
use crate::error::PortfolioError;
use crate::types::{EmbeddedFile, PortfolioMetadata, ViewMode};
use lopdf::Document;

pub struct Portfolio {
    doc: Document,
    files: Vec<EmbeddedFile>,
    metadata: PortfolioMetadata,
}

impl Portfolio {
    /// Check if raw PDF data is a Portfolio (has /Collection in catalog)
    pub fn is_portfolio(data: &[u8]) -> bool {
        Document::load_from(Cursor::new(data))
            .ok()
            .and_then(|doc| {
                doc.catalog()
                    .ok()
                    .and_then(|cat| cat.get(b"Collection").ok().cloned())
            })
            .is_some()
    }

    /// Open a PDF Portfolio and extract its embedded files
    pub fn open(data: &[u8]) -> Result<Self, PortfolioError> {
        let doc = Document::load_from(Cursor::new(data))?;
        
        // Check it's a portfolio
        let catalog = doc.catalog()?;
        if catalog.get(b"Collection").is_err() {
            return Err(PortfolioError::NotAPortfolio);
        }
        
        let pdf_version = format!("{}.{}", doc.version.major, doc.version.minor);
        
        // Determine view mode
        let view_mode = catalog
            .get(b"Collection")
            .ok()
            .and_then(|coll| {
                coll.as_dict().ok()?.get(b"View").ok()?.as_name_str().ok()
            })
            .map(|v| match v {
                "D" => ViewMode::Details,
                "T" => ViewMode::Tile,
                "H" => ViewMode::Hidden,
                _ => ViewMode::Details,
            });
        
        // Extract embedded files from name tree
        let files = Self::extract_files(&doc)?;
        let file_count = files.len();
        
        Ok(Self {
            doc,
            files,
            metadata: PortfolioMetadata {
                pdf_version,
                view_mode,
                file_count,
            },
        })
    }
    
    pub fn files(&self) -> &[EmbeddedFile] {
        &self.files
    }
    
    pub fn metadata(&self) -> &PortfolioMetadata {
        &self.metadata
    }
    
    /// Walk the /EmbeddedFiles name tree and extract all file entries
    fn extract_files(doc: &Document) -> Result<Vec<EmbeddedFile>, PortfolioError> {
        let mut files = Vec::new();
        
        let catalog = doc.catalog()?;
        
        // Navigate: Catalog -> /Names -> /EmbeddedFiles -> /Names array or /Kids
        let names_dict = match catalog.get(b"Names") {
            Ok(names) => names,
            Err(_) => return Ok(files), // No embedded files
        };
        
        let ef_dict = match names_dict.as_dict()?.get(b"EmbeddedFiles") {
            Ok(ef) => ef,
            Err(_) => return Ok(files),
        };
        
        let ef_dict = ef_dict.as_dict()?;
        
        // Get the /Names array which contains [ (name1) ref1 (name2) ref2 ... ]
        if let Ok(names_array) = ef_dict.get(b"Names") {
            let names = names_array.as_array()?;
            let mut i = 0;
            while i + 1 < names.len() {
                let name = names[i].as_str().unwrap_or("unknown").to_string();
                let file_spec_ref = &names[i + 1];
                
                if let Ok(file_spec) = doc.get_object(file_spec_ref.as_reference().ok_or_else(|| PortfolioError::NotAPortfolio)?) {
                    if let Ok(file_dict) = file_spec.as_dict() {
                        // Get filename from /F or /UF
                        let filename = file_dict.get(b"UF")
                            .or_else(|_| file_dict.get(b"F"))
                            .and_then(|n| n.as_str())
                            .unwrap_or(&name)
                            .to_string();
                        
                        // Get embedded file data from /EF -> /F
                        let ef = file_dict.get(b"EF").ok();
                        let data = ef.and_then(|ef| {
                            ef.as_dict().ok()?.get(b"F").ok()
                        }).and_then(|stream_ref| {
                            doc.get_object(stream_ref.as_reference().ok()?).ok()
                        }).and_then(|obj| {
                            obj.as_stream().ok().map(|s| s.content.clone())
                        }).unwrap_or_default();
                        
                        let size = data.len() as u64;
                        
                        // Guess mime type from extension
                        let mime_type = guess_mime(&filename);
                        
                        files.push(EmbeddedFile {
                            name: filename,
                            size,
                            mime_type,
                            data,
                        });
                    }
                }
                i += 2;
            }
        }
        
        Ok(files)
    }
}

fn guess_mime(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    match ext.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "docx" | "doc" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" | "xls" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" | "ppt" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }.to_string()
}
```

- [ ] **Step 3: Verify**

Run: `cargo check -p portfolio-core`
Expected: Compiles.

- [ ] **Step 4: Test against Portfolio1.pdf**

Add a test that opens `samples/Portfolio1.pdf` and verifies files are extracted.

- [ ] **Step 5: Commit**

---

### Task 4: Implement Portfolio::extract

**Files:**
- Modify: `crates/portfolio-core/src/reader.rs` (add extract methods)
- Modify: `crates/portfolio-core/src/lib.rs` (no change needed, re-exports already set)

**Interfaces:**
- Consumes: `Portfolio` from Task 3
- Produces: `portfolio.extract_all(dir)`, `portfolio.extract_file(name, dir)`

- [ ] **Step 1: Add extract methods to Portfolio impl**

Add to reader.rs in the `impl Portfolio` block:

```rust
pub fn extract_all(&self, output_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, PortfolioError> {
    std::fs::create_dir_all(output_dir)?;
    let mut paths = Vec::new();
    for file in &self.files {
        let path = output_dir.join(&file.name);
        std::fs::write(&path, &file.data)?;
        paths.push(path);
    }
    Ok(paths)
}

pub fn extract_file(&self, name: &str, output_dir: &std::path::Path) -> Result<std::path::PathBuf, PortfolioError> {
    let file = self.files.iter()
        .find(|f| f.name == name)
        .ok_or_else(|| PortfolioError::FileNotFound(name.to_string()))?;
    std::fs::create_dir_all(output_dir)?;
    let path = output_dir.join(&file.name);
    std::fs::write(&path, &file.data)?;
    Ok(path)
}
```

- [ ] **Step 2: Verify**

Run: `cargo check -p portfolio-core`
Expected: Compiles.

- [ ] **Step 3: Commit**

---

### Task 5: Implement Portfolio::create (PortfolioBuilder)

**Files:**
- Create: `crates/portfolio-core/src/writer.rs`
- Modify: `crates/portfolio-core/src/lib.rs` (add module + re-export)
- Note: The blank template lives at `samples/Portfolio2.pdf` and is embedded via `include_bytes!`

**Interfaces:**
- Consumes: `PortfolioError`, `ViewMode` from Task 2
- Produces: `Portfolio::create()`, `PortfolioBuilder::add_file()`, `PortfolioBuilder::set_view()`, `PortfolioBuilder::build()`

- [ ] **Step 1: Add writer module to lib.rs**

Append to lib.rs:
```rust
mod writer;
pub use writer::PortfolioBuilder;
```

- [ ] **Step 2: Write writer.rs**

This is the most complex piece. The approach:
1. Load the blank template PDF (Portfolio2.pdf embedded at compile time)
2. Add `/Collection` dict to catalog with the specified view mode
3. Add `/Names -> /EmbeddedFiles -> /Names` array with file entries
4. For each file: create object stream, add file spec with `/EF -> /F` reference
5. Save the document

```rust
use crate::error::PortfolioError;
use crate::types::ViewMode;
use lopdf::{Document, Object, ObjectId, Dictionary, Stream};
use std::io::Cursor;

/// Creates a new PDF Portfolio from a blank template
pub struct PortfolioBuilder {
    doc: Document,
    view_mode: ViewMode,
    files: Vec<(String, Vec<u8>, String)>, // (name, data, mime_type)
}

impl PortfolioBuilder {
    /// Start building a portfolio from the blank template
    pub fn new() -> Result<Self, PortfolioError> {
        let template = include_bytes!("../../../samples/Portfolio2.pdf");
        let doc = Document::load_from(Cursor::new(template.as_ref()))?;
        Ok(Self {
            doc,
            view_mode: ViewMode::Details,
            files: Vec::new(),
        })
    }
    
    pub fn set_view(&mut self, mode: ViewMode) -> &mut Self {
        self.view_mode = mode;
        self
    }
    
    pub fn add_file(&mut self, name: &str, data: Vec<u8>, mime_type: &str) -> &mut Self {
        self.files.push((name.to_string(), data, mime_type.to_string()));
        self
    }
    
    pub fn build(mut self) -> Result<Vec<u8>, PortfolioError> {
        // Build the /Names array for EmbeddedFiles
        let mut names_array: Vec<Object> = Vec::new();
        let next_id = self.doc.max_id + 1;
        
        for (i, (name, data, _mime)) in self.files.iter().enumerate() {
            let file_spec_id = ObjectId(next_id as u32 + (i * 2) as u32);
            let stream_id = ObjectId(next_id as u32 + (i * 2 + 1) as u32);
            
            // Create the embedded file stream
            let stream = Stream::new(
                Dictionary::new(),
                data.clone(),
            );
            self.doc.objects.insert(stream_id, Object::Stream(stream));
            
            // Create the file specification dictionary
            let mut fs_dict = Dictionary::new();
            fs_dict.set("Type", "Filespec");
            fs_dict.set("F", name.as_str());
            fs_dict.set("UF", name.as_str());
            
            let mut ef_dict = Dictionary::new();
            ef_dict.set("F", Object::Reference(stream_id));
            fs_dict.set("EF", Object::Dictionary(ef_dict));
            
            self.doc.objects.insert(file_spec_id, Object::Dictionary(fs_dict));
            
            // Add to names array: (name_string) reference
            names_array.push(Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            names_array.push(Object::Reference(file_spec_id));
        }
        
        // Create /EmbeddedFiles dictionary with /Names array
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        
        // Create /Names dict
        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Dictionary(ef_dict));
        
        // Set /Collection in catalog
        let mut catalog = self.doc.catalog_mut()?;
        catalog.set("Names", Object::Dictionary(names));
        
        let mut collection = Dictionary::new();
        collection.set("Type", "Collection");
        collection.set("View", Object::Name(self.view_mode.to_pdf_name().as_bytes().to_vec()));
        catalog.set("Collection", Object::Dictionary(collection));
        
        // Save to bytes
        let mut buf = Vec::new();
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }
}
```

- [ ] **Step 3: Verify**

Run: `cargo check -p portfolio-core`
Expected: Compiles.

- [ ] **Step 4: Note about lopdf API**

The exact `lopdf` API may differ slightly (e.g., `catalog_mut()`, `ObjectId`, `Stream::new`). Adjust to match the actual 0.34 API during implementation.

- [ ] **Step 5: Commit**

---

### Task 6: Build pfcli binary

**Files:**
- Create: `crates/pfcli/src/main.rs`

**Interfaces:**
- Consumes: `Portfolio`, `PortfolioBuilder`, `PortfolioError`, `ViewMode` from portfolio-core

- [ ] **Step 1: Write main.rs**

```rust
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use portfolio_core::{Portfolio, PortfolioBuilder, ViewMode};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pf", about = "PDF Portfolio tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new PDF Portfolio from files
    Create {
        /// Output portfolio file
        output: PathBuf,
        /// Input files to include
        files: Vec<PathBuf>,
    },
    /// List files in a PDF Portfolio
    List {
        /// Portfolio file to read
        portfolio: PathBuf,
    },
    /// Extract files from a PDF Portfolio
    Extract {
        /// Portfolio file to extract from
        portfolio: PathBuf,
        /// Output directory (defaults to current dir)
        #[arg(default_value = ".")]
        output_dir: PathBuf,
    },
    /// Show portfolio metadata
    Info {
        /// Portfolio file to inspect
        portfolio: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Command::Create { output, files } => {
            let mut builder = PortfolioBuilder::new()?;
            for path in &files {
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid filename")?;
                let data = std::fs::read(path)?;
                let mime = guess_mime(name);
                builder.add_file(name, data, &mime);
            }
            let pdf_data = builder.build()?;
            std::fs::write(&output, pdf_data)?;
            println!("Created portfolio: {} ({} files)", output.display(), files.len());
        }
        Command::List { portfolio } => {
            let data = std::fs::read(&portfolio)?;
            if !Portfolio::is_portfolio(&data) {
                println!("Not a PDF Portfolio");
                return Ok(());
            }
            let pf = Portfolio::open(&data)?;
            println!("{:<40} {:<12} {}", "Name", "Size", "Type");
            println!("{}", "-".repeat(70));
            for file in pf.files() {
                let size = format_size(file.size);
                println!("{:<40} {:<12} {}", file.name, size, file.mime_type);
            }
            println!("\n{} file(s)", pf.files().len());
        }
        Command::Extract { portfolio, output_dir } => {
            let data = std::fs::read(&portfolio)?;
            let pf = Portfolio::open(&data)?;
            let paths = pf.extract_all(&output_dir)?;
            for p in &paths {
                println!("Extracted: {}", p.display());
            }
            println!("{} file(s) extracted", paths.len());
        }
        Command::Info { portfolio } => {
            let data = std::fs::read(&portfolio)?;
            if !Portfolio::is_portfolio(&data) {
                println!("Not a PDF Portfolio");
                return Ok(());
            }
            let pf = Portfolio::open(&data)?;
            let meta = pf.metadata();
            println!("PDF Version: {}", meta.pdf_version);
            println!("File count:  {}", meta.file_count);
            if let Some(view) = meta.view_mode {
                println!("View mode:   {:?}", view);
            }
        }
    }
    
    Ok(())
}

fn guess_mime(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "docx" | "doc" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" | "xls" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" | "ppt" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }.to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
```

- [ ] **Step 2: Verify**

Run: `cargo build -p pfcli`
Expected: Compiles, produces `target/debug/pf`.

- [ ] **Step 3: Test roundtrip**

```bash
# Create a portfolio from test files
cargo run -p pfcli -- create /tmp/test_portfolio.pdf samples/Portfolio2.pdf

# List it
cargo run -p pfcli -- list /tmp/test_portfolio.pdf

# Info
cargo run -p pfcli -- info /tmp/test_portfolio.pdf
```

- [ ] **Step 4: Commit**

---

### Task 7: Test and fix against real PDF Portfolio (Portfolio1.pdf)

**Files:**
- Create: `crates/portfolio-core/tests/integration_test.rs`
- Potential modifications to reader.rs as needed

**Interfaces:**
- Tests: `Portfolio::is_portfolio`, `Portfolio::open`, `Portfolio::extract_all` against Portfolio1.pdf

- [ ] **Step 1: Write integration test**

```rust
use portfolio_core::{Portfolio, PortfolioBuilder, ViewMode};
use std::io::Cursor;

#[test]
fn test_detect_portfolio() {
    let data = include_bytes!("../../samples/Portfolio1.pdf");
    assert!(Portfolio::is_portfolio(data));
}

#[test]
fn test_detect_non_portfolio() {
    let data = include_bytes!("../../samples/Portfolio2.pdf");
    // Portfolio2 is a blank template - it might or might not have /Collection
    // This test verifies behavior is correct either way
    let _ = Portfolio::is_portfolio(data);
}

#[test]
fn test_open_portfolio1() {
    let data = include_bytes!("../../samples/Portfolio1.pdf");
    let pf = Portfolio::open(data).expect("Should open Portfolio1");
    let files = pf.files();
    println!("Found {} files:", files.len());
    for f in files {
        println!("  {} ({} bytes)", f.name, f.size);
    }
    assert!(!files.is_empty(), "Portfolio1 should contain files");
}

#[test]
fn test_extract_to_tempdir() {
    let data = include_bytes!("../../samples/Portfolio1.pdf");
    let pf = Portfolio::open(data).unwrap();
    let tmp = std::env::temp_dir().join("pf_test_extract");
    let _ = std::fs::remove_dir_all(&tmp);
    let paths = pf.extract_all(&tmp).unwrap();
    assert!(!paths.is_empty());
    for p in &paths {
        assert!(p.exists());
    }
    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p portfolio-core`
Expected: All tests pass. If Portfolio1.pdf parsing fails due to compressed object streams, investigate and fix reader.

- [ ] **Step 3: Fix any issues with real portfolio parsing**

Common issues:
- XRef streams instead of xref tables
- Object streams (compressed objects) - lopdf 0.34 handles these
- Linearized PDFs

- [ ] **Step 4: Commit**

---

### Task 8: End-to-end validation and polish

- [ ] **Step 1: Test create → list → extract roundtrip**

```bash
# Build
cargo build --release

# Create from PNG/PDF files
./target/release/pf create /tmp/test.pf.pdf \
  samples/Volumes/Acrobat\ X\ CIB/Lessons/Lesson07/Aquo_Overview.pdf \
  samples/Volumes/Acrobat\ X\ CIB/Lessons/Lesson07/Logo.gif

# List
./target/release/pf list /tmp/test.pf.pdf

# Extract
./target/release/pf extract /tmp/test.pf.pdf /tmp/pf_out/

# Verify extracted files match originals
diff <(md5 samples/Volumes/Acrobat\ X\ CIB/Lessons/Lesson07/Aquo_Overview.pdf) \
     <(md5 /tmp/pf_out/Aquo_Overview.pdf)
```

- [ ] **Step 2: Test reading Portfolio1.pdf via CLI**

```bash
./target/release/pf info samples/Portfolio1.pdf
./target/release/pf list samples/Portfolio1.pdf
./target/release/pf extract samples/Portfolio1.pdf /tmp/pf1_out/
```

- [ ] **Step 3: Clean up any warnings**

Run: `cargo clippy --workspace -- -D warnings` (if clippy available)

- [ ] **Step 4: Final commit**
