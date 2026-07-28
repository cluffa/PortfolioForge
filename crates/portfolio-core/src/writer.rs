use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;
use crate::types::ViewMode;

/// Builder for creating new PDF Portfolios. Creates a clean, minimal structure.
pub struct PortfolioBuilder {
    view_mode: ViewMode,
    files: Vec<(String, Vec<u8>)>,
}

impl PortfolioBuilder {
    /// Create a fresh portfolio builder (no template)
    pub fn new() -> Result<Self, PortfolioError> {
        Ok(Self {
            view_mode: ViewMode::Tile,
            files: Vec::new(),
        })
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

        let mut next_id: u32 = 1;

        // Create file entries: each file gets an EmbeddedFile stream and a Filespec dict
        let mut names_array: Vec<Object> = Vec::new();
        for (name, data) in &self.files {
            let stream_id = (next_id, 0u16);
            next_id += 1;
            let spec_id = (next_id, 0u16);
            next_id += 1;

            // EmbeddedFile stream
            let mut stream_dict = Dictionary::new();
            stream_dict.set("Type", "EmbeddedFile");
            let mut params = Dictionary::new();
            params.set("Size", Object::Integer(data.len() as i64));
            let now = chrono::Utc::now();
            params.set(
                "ModDate",
                Object::String(
                    now.format("D:%Y%m%d%H%M%SZ").to_string().into_bytes(),
                    lopdf::StringFormat::Literal,
                ),
            );
            stream_dict.set("Params", Object::Dictionary(params));
            doc.objects
                .insert(stream_id, Object::Stream(Stream::new(stream_dict, data.clone())));

            // Filespec dict
            let mut fs = Dictionary::new();
            fs.set("Type", "Filespec");
            fs.set(
                "F",
                Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            fs.set(
                "UF",
                Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            // Description: use filename stem as display title
            let desc = std::path::Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            fs.set(
                "Desc",
                Object::String(desc.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            );
            let mut ef = Dictionary::new();
            ef.set("F", Object::Reference(stream_id));
            fs.set("EF", Object::Dictionary(ef));
            doc.objects.insert(spec_id, Object::Dictionary(fs));

            names_array.push(Object::String(
                name.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            ));
            names_array.push(Object::Reference(spec_id));
        }

        // /EmbeddedFiles name tree
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        let ef_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(ef_id, Object::Dictionary(ef_dict));

        // /Names dict
        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Reference(ef_id));
        let names_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(names_id, Object::Dictionary(names));

        // /Collection dict with basic Schema for display
        let mut collection = Dictionary::new();
        collection.set("Type", "Collection");
        collection.set(
            "View",
            Object::Name(self.view_mode.to_pdf_name().as_bytes().to_vec()),
        );
        // Schema: define fields Acrobat displays in the portfolio view
        collection.set(
            "Schema",
            Object::Array(vec![
                field("FileName", "Name", 1),
                field("Description", "Desc", 2),
                field("ModifiedDate", "ModDate", 3),
                field("Size", "Size", 4),
            ]),
        );
        // Sort by file name by default
        let mut sort = Dictionary::new();
        sort.set("S", Object::Name(b"FileName".to_vec()));
        collection.set("Sort", Object::Dictionary(sort));
        let coll_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(coll_id, Object::Dictionary(collection));

        // Catalog
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Collection", Object::Reference(coll_id));
        catalog.set("Names", Object::Reference(names_id));

        // Minimal page (required by PDF spec, even for portfolios)
        let pages_id = (next_id, 0u16);
        next_id += 1;
        let page_id = (next_id, 0u16);
        next_id += 1;
        let content_id = (next_id, 0u16);
        next_id += 1;

        // Content stream
        let content_stream = Stream::new(Dictionary::new(), b" ".to_vec());
        doc.objects.insert(content_id, Object::Stream(content_stream));

        // Page
        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![
            Object::Integer(0), Object::Integer(0),
            Object::Integer(612), Object::Integer(792),
        ]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));

        // Pages
        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        catalog.set("Pages", Object::Reference(pages_id));
        let cat_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(cat_id, Object::Dictionary(catalog));

        // Trailer
        doc.trailer.set("Root", Object::Reference(cat_id));
        doc.max_id = next_id - 1;

        // Force cross-reference stream (cleaner output)
        doc.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceStream;

        let mut buf = Vec::new();
        doc.save_to(&mut buf)?;
        Ok(buf)
    }
}

/// Create a Collection Schema field entry
fn field(name: &str, key: &str, order: i64) -> Object {
    let mut d = Dictionary::new();
    d.set("N", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    d.set("O", Object::Integer(order));
    d.set("T", Object::Name(format!("adobe:{}", key).as_bytes().to_vec()));
    Object::Dictionary(d)
}
