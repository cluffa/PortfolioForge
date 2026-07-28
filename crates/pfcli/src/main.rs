use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use portfolio_core::{Portfolio, PortfolioBuilder};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pf", about = "PDF Portfolio tool — create, list, extract")]
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
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid filename")?;
                let data = std::fs::read(path)?;
                let mime = guess_mime(name);
                builder.add_file(name, data, &mime);
            }
            let pdf_data = builder.build()?;
            std::fs::write(&output, pdf_data)?;
            println!(
                "Created portfolio: {} ({} files)",
                output.display(),
                files.len()
            );
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
        Command::Extract {
            portfolio,
            output_dir,
        } => {
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
        "docx" | "doc" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "xlsx" | "xls" => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        "pptx" | "ppt" => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
    .to_string()
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
