use image::GenericImageView;
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

    #[error("Unsupported image format")]
    UnsupportedFormat,
}

/// Convert an image (PNG, JPG, TIFF, WebP, BMP, GIF) to a single-page PDF.
/// Returns the PDF file bytes.
pub fn image_to_pdf(data: &[u8]) -> Result<Vec<u8>, ConvertError> {
    let img = image::load_from_memory(data)?;
    let (width, height) = img.dimensions();
    let color = img.color();

    let (color_space, _bytes_per_pixel, filter_name, samples) = match color {
        image::ColorType::Rgb8 => ("DeviceRGB", 3, "DCTDecode", img.to_rgb8().into_raw()),
        image::ColorType::Rgba8 => {
            // Convert RGBA to RGB by flattening onto white background
            let rgb = img.to_rgba8();
            let mut flat = Vec::with_capacity((width * height * 3) as usize);
            for pixel in rgb.chunks(4) {
                let r = pixel[0] as f32 / 255.0;
                let g = pixel[1] as f32 / 255.0;
                let b = pixel[2] as f32 / 255.0;
                let a = pixel[3] as f32 / 255.0;
                flat.push((r * a * 255.0) as u8);
                flat.push((g * a * 255.0) as u8);
                flat.push((b * a * 255.0) as u8);
            }
            ("DeviceRGB", 3, "FlateDecode", flat)
        }
        image::ColorType::L8 | image::ColorType::La8 => {
            let gray = img.to_luma8().into_raw();
            ("DeviceGray", 1, "FlateDecode", gray)
        }
        image::ColorType::L16 => {
            // Convert 16-bit grayscale to 8-bit
            let gray16 = img.to_luma16();
            let gray8: Vec<u8> = gray16
                .pixels()
                .map(|p| (p.0[0] >> 8) as u8)
                .collect();
            ("DeviceGray", 1, "FlateDecode", gray8)
        }
        image::ColorType::Rgb16 => {
            let rgb16 = img.to_rgb16();
            let rgb8: Vec<u8> = rgb16
                .pixels()
                .flat_map(|p| [p.0[0] >> 8, p.0[1] >> 8, p.0[2] >> 8])
                .map(|v| v as u8)
                .collect();
            ("DeviceRGB", 3, "FlateDecode", rgb8)
        }
        _ => return Err(ConvertError::UnsupportedFormat),
    };

    // If DCTDecode is requested, verify the source was actually JPEG
    let (actual_filter, encoded_data) = if filter_name == "DCTDecode" {
        // For JPEG source, pass through the original bytes (already DCT compressed)
        // But only if the image crate confirms it was JPEG
        if let Some(jpeg_bytes) = detect_jpeg(data) {
            ("DCTDecode", jpeg_bytes.to_vec())
        } else {
            // Fall back to FlateDecode
            ("FlateDecode", deflate_compress(&samples))
        }
    } else {
        ("FlateDecode", deflate_compress(&samples))
    };

    // Build PDF
    let mut doc = lopdf::Document::new();
    doc.version = "1.7".to_string();

    // Image XObject
    let mut image_dict = Dictionary::new();
    image_dict.set("Type", "Name");
    image_dict.set("Subtype", "Image");
    image_dict.set("Width", Object::Integer(width as i64));
    image_dict.set("Height", Object::Integer(height as i64));
    image_dict.set("ColorSpace", Object::Name(color_space.as_bytes().to_vec()));
    image_dict.set("BitsPerComponent", Object::Integer(8));
    image_dict.set("Filter", Object::Name(actual_filter.as_bytes().to_vec()));

    let image_stream = Stream::new(image_dict, encoded_data);
    let image_id = (1, 0u16);
    doc.objects.insert(image_id, Object::Stream(image_stream));

    // Page content stream: draw the image at its natural size
    // PDF coordinate system: origin is bottom-left, units are points (1/72 inch)
    let content = format!(
        "q\n{} 0 0 {} 0 0 cm\n/Im0 Do\nQ",
        width, height
    );
    let content_stream = Stream::new(Dictionary::new(), content.into_bytes());
    let content_id = (2, 0u16);
    doc.objects
        .insert(content_id, Object::Stream(content_stream));

    // Resources dictionary
    let mut xobject_dict = Dictionary::new();
    xobject_dict.set("Im0", Object::Reference(image_id));

    let mut resources = Dictionary::new();
    resources.set("XObject", Object::Dictionary(xobject_dict));

    // Page object
    let mut page = Dictionary::new();
    page.set("Type", "Page");
    page.set("Parent", Object::Reference((3, 0u16)));
    page.set("MediaBox", Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(width as i64),
        Object::Integer(height as i64),
    ]));
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(resources));
    let page_id = (4, 0u16);
    doc.objects.insert(page_id, Object::Dictionary(page));

    // Pages object
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

/// Try to extract original JPEG bytes from the data (for pass-through)
fn detect_jpeg(data: &[u8]) -> Option<&[u8]> {
    if data.len() > 2 && data[0] == 0xFF && data[1] == 0xD8 {
        Some(data)
    } else {
        None
    }
}

/// Simple deflate compression
fn deflate_compress(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).ok();
    encoder.finish().unwrap_or_default()
}
