use image_converter::image_to_pdf;
use std::io::Cursor;

#[test]
fn test_png_to_pdf() {
    // Create a small in-memory PNG
    let img = image::ImageBuffer::from_fn(100, 100, |x, y| {
        image::Rgb([x as u8, y as u8, 128u8])
    });
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .unwrap();

    let pdf = image_to_pdf(&png_bytes).expect("Should convert PNG to PDF");
    assert!(!pdf.is_empty());

    // Verify it's a valid PDF
    let doc = lopdf::Document::load_from(Cursor::new(&pdf)).expect("Should load as PDF");
    assert!(doc.catalog().is_ok());
    assert_eq!(doc.version, "1.7");
}

#[test]
fn test_jpeg_to_pdf() {
    let img = image::ImageBuffer::from_fn(50, 50, |x, y| {
        image::Rgb([x as u8, y as u8, 200u8])
    });
    let mut jpg_bytes = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut jpg_bytes),
        image::ImageFormat::Jpeg,
    )
    .unwrap();

    let pdf = image_to_pdf(&jpg_bytes).expect("Should convert JPEG to PDF");
    assert!(!pdf.is_empty());
    let doc = lopdf::Document::load_from(Cursor::new(&pdf)).expect("Should load as PDF");
    assert!(doc.catalog().is_ok());
}

#[test]
fn test_real_image() {
    // Test with the sample Logo.gif from the lesson files
    let data = include_bytes!("../../../samples/Volumes/Acrobat X CIB/Lessons/Lesson07/Logo.gif");
    let pdf = image_to_pdf(data).expect("Should convert GIF to PDF");
    assert!(pdf.len() > 1000);
    let doc = lopdf::Document::load_from(Cursor::new(&pdf)).expect("Should load as PDF");
    assert!(doc.catalog().is_ok());
}
