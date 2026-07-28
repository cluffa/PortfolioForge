use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;
use crate::types::ViewMode;

/// Builder for creating new PDF Portfolios
pub struct PortfolioBuilder {
    doc: lopdf::Document,
    view_mode: ViewMode,
    files: Vec<(String, Vec<u8>, String)>, // (name, data, mime_type)
}

impl PortfolioBuilder {
    /// Create a fresh portfolio builder with a minimal blank PDF page
    pub fn new() -> Result<Self, PortfolioError> {
        let mut doc = lopdf::Document::new();

        // Create a minimal blank A4 page
        let page_id = (1, 0u16);
        let pages_id = (2, 0u16);
        let catalog_id = (3, 0u16);

        // Content stream for a blank page
        let content = b"q\nBT\n/F1 24 Tf\n100 700 Td\n(PortfolioForge) Tj\nET\nQ\n";
        let mut content_stream = Stream::new(Dictionary::new(), content.to_vec());
        content_stream.allows_compression = true;
        doc.objects
            .insert((4, 0u16), Object::Stream(content_stream));

        // Page object
        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ]));
        page.set("Contents", Object::Reference((4, 0u16)));
        page.set(
            "Resources",
            Object::Dictionary({
                let mut res = Dictionary::new();
                res.set(
                    "Font",
                    Object::Dictionary({
                        let mut fonts = Dictionary::new();
                        fonts.set(
                            "F1",
                            Object::Dictionary({
                                let mut f1 = Dictionary::new();
                                f1.set("Type", "Font");
                                f1.set("Subtype", "Type1");
                                f1.set("BaseFont", "Helvetica");
                                f1
                            }),
                        );
                        fonts
                    }),
                );
                res
            }),
        );
        doc.objects.insert(page_id, Object::Dictionary(page));

        // Pages object
        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set(
            "Kids",
            Object::Array(vec![Object::Reference(page_id)]),
        );
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        // Catalog object
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", Object::Reference(pages_id));
        doc.objects.insert(catalog_id, Object::Dictionary(catalog));

        // Set trailer root
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.max_id = 4;

        Ok(Self {
            doc,
            view_mode: ViewMode::Details,
            files: Vec::new(),
        })
    }

    /// Set the view mode for the portfolio collection
    pub fn set_view(&mut self, mode: ViewMode) -> &mut Self {
        self.view_mode = mode;
        self
    }

    /// Add a file to the portfolio
    pub fn add_file(&mut self, name: &str, data: Vec<u8>, _mime_type: &str) -> &mut Self {
        self.files
            .push((name.to_string(), data, _mime_type.to_string()));
        self
    }

    /// Build the portfolio and return the PDF bytes
    pub fn build(mut self) -> Result<Vec<u8>, PortfolioError> {
        let mut names_array: Vec<Object> = Vec::new();
        let mut next_id = self.doc.max_id + 1;

        for (name, data, _mime) in &self.files {
            let file_spec_id = (next_id, 0u16);
            let stream_id = (next_id + 1, 0u16);
            next_id += 2;

            // Embedded file stream
            let mut stream_dict = Dictionary::new();
            stream_dict.set("Type", "EmbeddedFile");
            stream_dict.set(
                "Params",
                Object::Dictionary({
                    let mut params = Dictionary::new();
                    params.set("Size", Object::Integer(data.len() as i64));
                    params
                }),
            );

            let mut stream = Stream::new(stream_dict, data.clone());
            stream.allows_compression = false;
            self.doc.objects.insert(stream_id, Object::Stream(stream));

            // File specification dictionary
            let mut fs_dict = Dictionary::new();
            fs_dict.set("Type", "Filespec");
            fs_dict.set("F", name.as_str());
            fs_dict.set("UF", name.as_str());

            let mut ef_dict = Dictionary::new();
            ef_dict.set("F", Object::Reference(stream_id));
            fs_dict.set("EF", Object::Dictionary(ef_dict));

            self.doc
                .objects
                .insert(file_spec_id, Object::Dictionary(fs_dict));

            names_array.push(Object::String(
                name.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            ));
            names_array.push(Object::Reference(file_spec_id));
        }

        // /EmbeddedFiles dictionary
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        let ef_id = (next_id, 0u16);
        self.doc
            .objects
            .insert(ef_id, Object::Dictionary(ef_dict));

        // Add /Collection and /Names to catalog
        let catalog_id = self
            .doc
            .trailer
            .get(b"Root")
            .and_then(|o| o.as_reference())
            .map_err(|_| PortfolioError::ParseError(lopdf::Error::Type))?;

        let catalog = self.doc.get_dictionary_mut(catalog_id)?;

        let mut collection = Dictionary::new();
        collection.set("Type", "Collection");
        collection.set(
            "View",
            Object::Name(self.view_mode.to_pdf_name().as_bytes().to_vec()),
        );
        catalog.set("Collection", Object::Dictionary(collection));

        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Reference(ef_id));
        catalog.set("Names", Object::Dictionary(names));

        // Save to bytes
        let mut buf = Vec::new();
        // Update max_id to include all objects we added
        let max_obj_id = self.doc.objects.keys().map(|&(id, _)| id).max().unwrap_or(0);
        self.doc.max_id = max_obj_id;
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }
}
