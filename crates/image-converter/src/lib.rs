use lopdf::{Dictionary, Object, Stream};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("Failed to decode image: {0}")]
    Decode(#[from] image::ImageError),

    #[error("Failed to create PDF: {0}")]
    Pdf(#[from] lopdf::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to create image XObject: {0}")]
    XObject(String),
}

/// Convert an image (PNG, JPG, TIFF, WebP, BMP, GIF) to a single-page PDF.
/// Uses lopdf's built-in image handling for correct XObject construction.
pub fn image_to_pdf(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    image_to_pdf_with_dpi(data, 150.0)
}

/// Convert with a custom DPI (default 150 for good on-screen quality)
pub fn image_to_pdf_with_dpi(data: &[u8], dpi: f64) -> Result<Vec<u8>, ConvertError> {
    let image_stream = lopdf::xobject::image_from(data.to_vec())?;

    let px_w = image_stream
        .dict
        .get(b"Width")
        .and_then(|o| o.as_i64())
        .unwrap_or(100) as f64;
    let px_h = image_stream
        .dict
        .get(b"Height")
        .and_then(|o| o.as_i64())
        .unwrap_or(100) as f64;

    // Scale: pixels → points at given DPI (1 inch = 72 points)
    let scale = 72.0 / dpi;
    let pt_w = px_w * scale;
    let pt_h = px_h * scale;

    let mut doc = lopdf::Document::new();
    doc.version = "1.7".to_string();

    let image_id = (1, 0u16);
    doc.objects.insert(image_id, Object::Stream(image_stream));

    // Draw image scaled to pt dimensions
    let content = format!("q\n{pt_w:.0} 0 0 {pt_h:.0} 0 0 cm\n/Im0 Do\nQ");
    let content_stream = Stream::new(Dictionary::new(), content.into_bytes());
    let content_id = (2, 0u16);
    doc.objects.insert(content_id, Object::Stream(content_stream));

    // Resources: reference the image XObject
    let mut xobject_dict = Dictionary::new();
    xobject_dict.set("Im0", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set("XObject", Object::Dictionary(xobject_dict));

    // Page
    let mut page = Dictionary::new();
    page.set("Type", "Page");
    page.set("Parent", Object::Reference((3, 0u16)));
    page.set("MediaBox", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(pt_w as i64),
        Object::Integer(pt_h as i64),
    ]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(resources));
    let page_id = (4, 0u16);
    doc.objects.insert(page_id, Object::Dictionary(page));

    // Pages
    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", Object::Integer(1));
    let pages_id = (3, 0u16);
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    // Catalog
    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = (5, 0u16);
    doc.objects.insert(catalog_id, Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.max_id = 5;

    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}
