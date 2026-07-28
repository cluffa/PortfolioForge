use std::io::Cursor;

use lopdf::{Dictionary, Object, Stream};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("Failed to parse DOCX: {0}")]
    Parse(String),

    #[error("Failed to create PDF: {0}")]
    Pdf(#[from] lopdf::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convert a DOCX file to a basic PDF with extracted text.
/// Does NOT preserve formatting — text-only extraction for V1.
pub fn docx_to_pdf(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let docx = docx_rust::DocxFile::from_reader(Cursor::new(data))
        .map_err(|e| ConvertError::Parse(format!("Failed to open DOCX: {}", e)))?;

    let parsed = docx
        .parse()
        .map_err(|e| ConvertError::Parse(format!("Failed to parse DOCX: {}", e)))?;

    let text = parsed.document.body.text();
    let lines: Vec<&str> = text.lines().collect();

    // Build a simple PDF with the text
    let mut doc = lopdf::Document::new();
    doc.version = "1.7".to_string();

    // Font: use Helvetica (built-in, no embedding needed)
    let mut fonts = Dictionary::new();
    let mut f1 = Dictionary::new();
    f1.set("Type", "Font");
    f1.set("Subtype", "Type1");
    f1.set("BaseFont", "Helvetica");
    fonts.set("F1", Object::Dictionary(f1));

    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(fonts));

    // Build content stream with one line per paragraph
    let font_size = 11.0;
    let leading = 14.0; // line spacing
    let margin_left = 72.0; // 1 inch
    let margin_top = 720.0; // 10 inches from bottom (letter size)

    let mut content = Vec::new();
    let mut y = margin_top;

    // Start text object
    content.extend_from_slice(format!(
        "BT\n/F1 {} Tf\n{} TL\n",
        font_size, leading
    ).as_bytes());

    for line in &lines {
        if y < 50.0 {
            break; // Stop if we run off the page
        }
        // Escape PDF string special chars: \ ( ) 
        let escaped = line
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        content.extend_from_slice(
            format!("1 0 0 1 {} {} Tm\n({}) Tj\nT*\n", margin_left, y, escaped).as_bytes(),
        );
        y -= leading;
    }
    content.extend_from_slice(b"ET\n");

    let content_stream = Stream::new(Dictionary::new(), content);
    let content_id = (1, 0u16);
    doc.objects
        .insert(content_id, Object::Stream(content_stream));

    // Page
    let mut page = Dictionary::new();
    page.set("Type", "Page");
    page.set("Parent", Object::Reference((2, 0u16)));
    page.set("MediaBox", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(612),
        Object::Integer(792),
    ]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(resources));
    let page_id = (3, 0u16);
    doc.objects.insert(page_id, Object::Dictionary(page));

    // Pages
    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", Object::Integer(1));
    let pages_id = (2, 0u16);
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    // Catalog
    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = (4, 0u16);
    doc.objects.insert(catalog_id, Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.max_id = 4;

    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}
