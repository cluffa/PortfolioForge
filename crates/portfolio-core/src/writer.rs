use std::io::Cursor;

use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;
use crate::types::ViewMode;

/// Builder for creating new PDF Portfolios using an Acrobat-generated template
pub struct PortfolioBuilder {
    doc: lopdf::Document,
    view_mode: ViewMode,
    files: Vec<(String, Vec<u8>, String)>, // (name, data, mime_type)
}

impl PortfolioBuilder {
    /// Start building a portfolio from the Acrobat-generated blank template
    pub fn new() -> Result<Self, PortfolioError> {
        let template = include_bytes!("../../../samples/Portfolio2.pdf");
        let doc = lopdf::Document::load_from(Cursor::new(template.as_ref()))?;
        Ok(Self {
            doc,
            view_mode: ViewMode::Tile,
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
                    // Add modification date in PDF format
                    let now = chrono::Utc::now();
                    let date_str = now.format("D:%Y%m%d%H%M%SZ").to_string();
                    params.set("ModDate", Object::String(
                        date_str.into_bytes(),
                        lopdf::StringFormat::Literal,
                    ));
                    params
                }),
            );

            let mut stream = Stream::new(stream_dict, data.clone());
            stream.allows_compression = true;
            self.doc.objects.insert(stream_id, Object::Stream(stream));

            // File specification dictionary
            let mut fs_dict = Dictionary::new();
            fs_dict.set("Type", "Filespec");
            fs_dict.set("F", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            fs_dict.set("UF", Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));

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

        // /EmbeddedFiles dictionary with /Names array
        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        let ef_id = (next_id, 0u16);
        self.doc
            .objects
            .insert(ef_id, Object::Dictionary(ef_dict));

        // Update the catalog — preserve existing Collection settings from template
        {
            // First, check if there's an existing Collection with an ID we can update
            let existing_coll_id = {
                let catalog = self.doc.catalog()?;
                catalog.get(b"Collection").ok()
                    .and_then(|o| o.as_reference().ok())
            };

            if let Some(coll_id) = existing_coll_id {
                // Update the existing Collection object's /View
                if let Ok(coll) = self.doc.get_object_mut(coll_id) {
                    if let Ok(dict) = coll.as_dict_mut() {
                        dict.set("View", Object::Name(
                            self.view_mode.to_pdf_name().as_bytes().to_vec(),
                        ));
                    }
                }
            } else {
                // No existing Collection — create one
                let catalog = self.doc.catalog_mut()?;
                let mut collection = Dictionary::new();
                collection.set("Type", "Collection");
                collection.set("View", Object::Name(
                    self.view_mode.to_pdf_name().as_bytes().to_vec(),
                ));
                catalog.set("Collection", Object::Dictionary(collection));
            }

            // Set /Names -> /EmbeddedFiles
            let catalog = self.doc.catalog_mut()?;
            let mut names = Dictionary::new();
            names.set("EmbeddedFiles", Object::Reference(ef_id));
            catalog.set("Names", Object::Dictionary(names));
        }

        // Save with updated max_id so new objects get included in xref
        let max_obj_id = self
            .doc
            .objects
            .keys()
            .map(|&(id, _)| id)
            .max()
            .unwrap_or(0);
        self.doc.max_id = max_obj_id;

        let mut buf = Vec::new();
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }
}
