pub mod editor;
pub mod error;
pub mod reader;
pub mod types;
pub mod writer;

pub use editor::PortfolioEditor;
pub use error::PortfolioError;
pub use reader::Portfolio;
pub use types::{EmbeddedFile, PortfolioMetadata, ViewMode};
pub use writer::PortfolioBuilder;

/// Validate a PDF Portfolio — checks structural integrity
pub fn validate(data: &[u8]) -> Result<Vec<String>, PortfolioError> {
    let mut issues = Vec::new();
    let doc = lopdf::Document::load_from(std::io::Cursor::new(data))?;

    // Check catalog
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => {
            issues.push("Missing or invalid document catalog".into());
            return Ok(issues);
        }
    };

    // Check /Collection
    match catalog.get(b"Collection") {
        Ok(_) => {}
        Err(_) => issues.push("Missing /Collection dictionary (not a portfolio)".into()),
    }

    // Check /Names → /EmbeddedFiles
    match catalog.get(b"Names") {
        Ok(names) => {
            if let Ok(names_dict) = names.as_dict() {
                match names_dict.get(b"EmbeddedFiles") {
                    Ok(_) => {}
                    Err(_) => issues.push("Missing /EmbeddedFiles name tree".into()),
                }
            } else {
                issues.push("/Names is not a dictionary".into());
            }
        }
        Err(_) => {
            // No files = empty portfolio is valid
        }
    }

    // Check for empty file entries
    if let Ok(pf) = Portfolio::open(data) {
        for file in pf.files() {
            if file.data.is_empty() {
                issues.push(format!("File '{}' has zero bytes", file.name));
            }
            if file.name.is_empty() {
                issues.push("Found file entry with empty name".into());
            }
        }
    }

    if issues.is_empty() {
        issues.push("Portfolio is valid".into());
    }
    Ok(issues)
}
