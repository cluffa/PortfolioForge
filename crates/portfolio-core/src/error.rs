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
