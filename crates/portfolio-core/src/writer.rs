use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;
use crate::types::ViewMode;

pub struct PortfolioBuilder {
    view_mode: ViewMode,
    files: Vec<(String, Vec<u8>)>,
}

impl PortfolioBuilder {
    pub fn new() -> Result<Self, PortfolioError> {
        Ok(Self { view_mode: ViewMode::Tile, files: Vec::new() })
    }

    pub fn set_view(&mut self, mode: ViewMode) -> &mut Self {
        self.view_mode = mode;
        self
    }

    pub fn add_file(&mut self, name: &str, data: Vec<u8>, _mime_type: &str) -> &mut Self {
        self.files.push((name.to_string(), data));
        self
    }

    pub fn build(self) -> Result<Vec<u8>, PortfolioError> {
        let mut doc = lopdf::Document::new();
        doc.version = "1.7".to_string();
        let mut next: u32 = 1;
        let mut alloc = || { let id = (next, 0u16); next += 1; id };

        // ── Cover page content ──
        let mut cover_text = String::from(
            "BT\n/F1 24 Tf\n72 720 Td\n(PortfolioForge) Tj\nT*\n"
        );
        cover_text.push_str("/F1 12 Tf\n0 -20 Td\n");
        cover_text.push_str("(This PDF Portfolio contains multiple documents.) Tj\nT*\n");
        cover_text.push_str("(For the best experience, open in Adobe Acrobat.) Tj\nT*\n");
        cover_text.push_str("0 -20 Td\n/F1 10 Tf\n(Contents:) Tj\nT*\n");
        for (i, (name, _)) in self.files.iter().enumerate() {
            cover_text.push_str(&format!(
                "({}. {}) Tj\nT*\n",
                i + 1,
                name.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
            ));
        }
        cover_text.push_str("ET\n");

        // Font: Helvetica
        let mut fonts = Dictionary::new();
        let mut f1 = Dictionary::new();
        f1.set("Type", "Font"); f1.set("Subtype", "Type1"); f1.set("BaseFont", "Helvetica");
        fonts.set("F1", Object::Dictionary(f1));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));

        let cover_stream = Stream::new(Dictionary::new(), cover_text.into_bytes());
        let content_id = alloc();
        doc.objects.insert(content_id, Object::Stream(cover_stream));

        let pages_id = alloc();
        let page_id = alloc();
        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![
            Object::Integer(0), Object::Integer(0),
            Object::Integer(612), Object::Integer(792),
        ]));
        page.set("Contents", Object::Reference(content_id));
        page.set("Resources", Object::Dictionary(resources));
        doc.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        // ── File entries ──
        let mut names_array: Vec<Object> = Vec::new();
        for (name, data) in &self.files {
            let stream_id = alloc();
            let spec_id = alloc();

            let mut sd = Dictionary::new();
            sd.set("Type", "EmbeddedFile");
            let mut params = Dictionary::new();
            params.set("Size", Object::Integer(data.len() as i64));
            let now = chrono::Utc::now();
            params.set("ModDate", Object::String(
                now.format("D:%Y%m%d%H%M%SZ").to_string().into_bytes(),
                lopdf::StringFormat::Literal,
            ));
            sd.set("Params", Object::Dictionary(params));
            let mut stream = Stream::new(sd, data.clone());
            stream.compress().ok();
            doc.objects.insert(stream_id, Object::Stream(stream));

            let mut fs = Dictionary::new();
            fs.set("Type", "Filespec");
            fs.set("F", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            fs.set("UF", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            let desc = std::path::Path::new(name).file_stem()
                .and_then(|s| s.to_str()).unwrap_or(name);
            fs.set("Desc", Object::String(desc.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            let mut ef = Dictionary::new();
            ef.set("F", Object::Reference(stream_id));
            fs.set("EF", Object::Dictionary(ef));
            doc.objects.insert(spec_id, Object::Dictionary(fs));

            names_array.push(Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            names_array.push(Object::Reference(spec_id));
        }

        // /EmbeddedFiles
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        let ef_id = alloc();
        doc.objects.insert(ef_id, Object::Dictionary(ef_dict));

        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Reference(ef_id));
        let names_id = alloc();
        doc.objects.insert(names_id, Object::Dictionary(names));

        // /Collection
        let mut collection = Dictionary::new();
        collection.set("Type", "Collection");
        collection.set("View", Object::Name(self.view_mode.to_pdf_name().as_bytes().to_vec()));
        // Schema: basic fields
        collection.set("Schema", Object::Array(vec![
            schema_field("FileName", "Name", 1),
            schema_field("Description", "Desc", 2),
            schema_field("Modified", "ModDate", 3),
            schema_field("Size", "Size", 4),
        ]));
        let mut sort = Dictionary::new();
        sort.set("S", Object::Name(b"FileName".to_vec()));
        collection.set("Sort", Object::Dictionary(sort));
        let coll_id = alloc();
        doc.objects.insert(coll_id, Object::Dictionary(collection));

        // Catalog
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", Object::Reference(pages_id));
        catalog.set("Collection", Object::Reference(coll_id));
        catalog.set("Names", Object::Reference(names_id));
        let cat_id = alloc();
        doc.objects.insert(cat_id, Object::Dictionary(catalog));

        doc.trailer.set("Root", Object::Reference(cat_id));
        doc.max_id = next - 1;
        doc.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceStream;

        let mut buf = Vec::new();
        doc.save_to(&mut buf)?;
        Ok(buf)
    }
}

fn schema_field(name: &str, key: &str, order: i64) -> Object {
    let mut d = Dictionary::new();
    d.set("N", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    d.set("O", Object::Integer(order));
    d.set("T", Object::Name(format!("adobe:{}", key).as_bytes().to_vec()));
    Object::Dictionary(d)
}
