use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use portfolio_core::{Portfolio, PortfolioBuilder, PortfolioEditor};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pf", about = "PDF Portfolio tool — create, list, extract, edit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new PDF Portfolio from files
    Create {
        output: PathBuf,
        files: Vec<PathBuf>,
    },
    /// List files in a PDF Portfolio
    List {
        portfolio: PathBuf,
    },
    /// Extract files from a PDF Portfolio
    Extract {
        portfolio: PathBuf,
        #[arg(default_value = ".")]
        output_dir: PathBuf,
    },
    /// Show portfolio metadata
    Info {
        portfolio: PathBuf,
    },
    /// Validate a PDF Portfolio structure
    Validate {
        portfolio: PathBuf,
    },
    /// Add files to an existing PDF Portfolio
    Add {
        portfolio: PathBuf,
        files: Vec<PathBuf>,
    },
    /// Remove a file from a PDF Portfolio
    Remove {
        portfolio: PathBuf,
        /// Name of the file to remove
        name: String,
    },
    /// Replace a file in a PDF Portfolio
    Replace {
        portfolio: PathBuf,
        /// Name of the file to replace
        old: String,
        /// Replacement file
        new: PathBuf,
        /// New name (optional, defaults to replacement file's name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Rename a file in a PDF Portfolio
    Rename {
        portfolio: PathBuf,
        old: String,
        new: String,
    },
    /// Reorder files in a PDF Portfolio
    Reorder {
        portfolio: PathBuf,
        /// Ordered list of file names (remaining files stay at end)
        names: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Create { output, files } => cmd_create(output, files)?,
        Command::List { portfolio } => cmd_list(portfolio)?,
        Command::Extract {
            portfolio,
            output_dir,
        } => cmd_extract(portfolio, output_dir)?,
        Command::Info { portfolio } => cmd_info(portfolio)?,
        Command::Validate { portfolio } => cmd_validate(portfolio)?,
        Command::Add { portfolio, files } => cmd_add(portfolio, files)?,
        Command::Remove { portfolio, name } => cmd_remove(portfolio, name)?,
        Command::Replace {
            portfolio,
            old,
            new,
            name,
        } => cmd_replace(portfolio, old, new, name)?,
        Command::Rename {
            portfolio,
            old,
            new,
        } => cmd_rename(portfolio, old, new)?,
        Command::Reorder { portfolio, names } => cmd_reorder(portfolio, names)?,
    }

    Ok(())
}

// ── Create ──

fn cmd_create(output: PathBuf, files: Vec<PathBuf>) -> Result<()> {
    let mut builder = PortfolioBuilder::new()?;
    for path in &files {
        let (final_name, final_data, mime) = convert_if_needed(path)?;
        builder.add_file(&final_name, final_data, &mime);
    }
    let pdf_data = builder.build()?;
    std::fs::write(&output, pdf_data)?;
    println!("Created portfolio: {} ({} files)", output.display(), files.len());
    Ok(())
}

// ── List ──

fn cmd_list(portfolio: PathBuf) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    if !Portfolio::is_portfolio(&data) {
        println!("Not a PDF Portfolio");
        return Ok(());
    }
    let pf = Portfolio::open(&data)?;
    if pf.files().is_empty() {
        println!("(empty portfolio)");
    } else {
        println!("{:<40} {:<12} {}", "Name", "Size", "Type");
        println!("{}", "-".repeat(70));
        for file in pf.files() {
            println!("{:<40} {:<12} {}", file.name, format_size(file.size), file.mime_type);
        }
    }
    println!("\n{} file(s)", pf.files().len());
    Ok(())
}

// ── Extract ──

fn cmd_extract(portfolio: PathBuf, output_dir: PathBuf) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let pf = Portfolio::open(&data)?;
    let paths = pf.extract_all(&output_dir)?;
    for p in &paths {
        println!("Extracted: {}", p.display());
    }
    println!("{} file(s) extracted", paths.len());
    Ok(())
}

// ── Info ──

fn cmd_info(portfolio: PathBuf) -> Result<()> {
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
    Ok(())
}

// ── Validate ──

fn cmd_validate(portfolio: PathBuf) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let issues = portfolio_core::validate(&data)?;
    for issue in &issues {
        println!("  {}", issue);
    }
    Ok(())
}

// ── Add ──

fn cmd_add(portfolio: PathBuf, files: Vec<PathBuf>) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let mut editor = PortfolioEditor::open(&data)?;
    for path in &files {
        let (name, file_data, _) = convert_if_needed(path)?;
        editor.add_file(&name, file_data)?;
    }
    let modified = editor.save()?;
    std::fs::write(&portfolio, modified)?;
    println!("Added {} file(s) to {}", files.len(), portfolio.display());
    cmd_list(portfolio)?;
    Ok(())
}

// ── Remove ──

fn cmd_remove(portfolio: PathBuf, name: String) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let mut editor = PortfolioEditor::open(&data)?;
    editor.remove_file(&name)?;
    let modified = editor.save()?;
    std::fs::write(&portfolio, modified)?;
    println!("Removed '{}' from {}", name, portfolio.display());
    Ok(())
}

// ── Replace ──

fn cmd_replace(
    portfolio: PathBuf,
    old: String,
    new: PathBuf,
    name: Option<String>,
) -> Result<()> {
    let new_name = name.unwrap_or_else(|| {
        new.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });
    let (_, file_data, _) = convert_if_needed(&new)?;
    let data = std::fs::read(&portfolio)?;
    let mut editor = PortfolioEditor::open(&data)?;
    editor.replace_file(&old, &new_name, file_data)?;
    let modified = editor.save()?;
    std::fs::write(&portfolio, modified)?;
    println!("Replaced '{}' → '{}' in {}", old, new_name, portfolio.display());
    Ok(())
}

// ── Rename ──

fn cmd_rename(portfolio: PathBuf, old: String, new: String) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let mut editor = PortfolioEditor::open(&data)?;
    editor.rename_file(&old, &new)?;
    let modified = editor.save()?;
    std::fs::write(&portfolio, modified)?;
    println!("Renamed '{}' → '{}' in {}", old, new, portfolio.display());
    Ok(())
}

// ── Reorder ──

fn cmd_reorder(portfolio: PathBuf, names: Vec<String>) -> Result<()> {
    let data = std::fs::read(&portfolio)?;
    let mut editor = PortfolioEditor::open(&data)?;
    let names_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    editor.reorder_files(&names_refs)?;
    let modified = editor.save()?;
    std::fs::write(&portfolio, modified)?;
    println!("Reordered files in {}", portfolio.display());
    cmd_list(portfolio)?;
    Ok(())
}

// ── Helpers ──

fn convert_if_needed(path: &std::path::Path) -> Result<(String, Vec<u8>, String)> {
    let original_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let data = std::fs::read(path)?;

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "webp" | "bmp" | "gif" => {
            let pdf_name = format!("{}.pdf", original_name);
            match image_converter::image_to_pdf(&data) {
                Ok(pdf_data) => Ok((pdf_name, pdf_data, "application/pdf".into())),
                Err(e) => {
                    eprintln!("Warning: failed to convert {}, adding as-is: {}", original_name, e);
                    Ok((original_name.into(), data, guess_mime(original_name)))
                }
            }
        }
        "docx" => {
            let pdf_name = format!("{}.pdf", original_name);
            match docx_converter::docx_to_pdf(&data) {
                Ok(pdf_data) => Ok((pdf_name, pdf_data, "application/pdf".into())),
                Err(e) => {
                    eprintln!("Warning: failed to convert {}, adding as-is: {}", original_name, e);
                    Ok((original_name.into(), data, guess_mime(original_name)))
                }
            }
        }
        _ => Ok((original_name.into(), data, guess_mime(original_name))),
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
