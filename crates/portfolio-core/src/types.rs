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
