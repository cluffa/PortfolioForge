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

        // /Collection dict
        let mut collection = Dictionary::new();
        collection.set("Type", "Collection");
        collection.set(
            "View",
            Object::Name(self.view_mode.to_pdf_name().as_bytes().to_vec()),
        );
        let coll_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(coll_id, Object::Dictionary(collection));

        // Catalog
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Collection", Object::Reference(coll_id));
        catalog.set("Names", Object::Reference(names_id));
        // Minimal empty page tree (required by PDF spec even for portfolios)
        let pages_id = (next_id, 0u16);
        next_id += 1;
        catalog.set("Pages", Object::Reference(pages_id));
        let cat_id = (next_id, 0u16);
        next_id += 1;
        doc.objects.insert(cat_id, Object::Dictionary(catalog));

        // Minimal Pages object
        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set("Kids", Object::Array(vec![]));
        pages.set("Count", Object::Integer(0));
        doc.objects.insert(pages_id, Object::Dictionary(pages));

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
