use std::io::Cursor;

use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;
use crate::types::ViewMode;

/// Builder for creating new PDF Portfolios using the Acrobat template for cover page
pub struct PortfolioBuilder {
    doc: lopdf::Document,
    files: Vec<(String, Vec<u8>)>,
}

impl PortfolioBuilder {
    /// Start building from the Acrobat template (has cover page, layout, schema)
    pub fn new() -> Result<Self, PortfolioError> {
        let template = include_bytes!("../../../samples/Portfolio2.pdf");
        let doc = lopdf::Document::load_from(Cursor::new(template.as_ref()))?;
        Ok(Self {
            doc,
            files: Vec::new(),
        })
    }

    pub fn set_view(&mut self, mode: ViewMode) -> &mut Self {
        // Update the existing Collection's /View
        if let Ok(catalog) = self.doc.catalog() {
            if let Ok(coll) = catalog.get(b"Collection") {
                if let Ok(coll_id) = coll.as_reference() {
                    if let Ok(obj) = self.doc.get_object_mut(coll_id) {
                        if let Ok(dict) = obj.as_dict_mut() {
                            dict.set(
                                "View",
                                Object::Name(mode.to_pdf_name().as_bytes().to_vec()),
                            );
                        }
                    }
                }
            }
        }
        self
    }

    pub fn add_file(&mut self, name: &str, data: Vec<u8>, _mime_type: &str) -> &mut Self {
        self.files.push((name.to_string(), data));
        self
    }

    pub fn build(mut self) -> Result<Vec<u8>, PortfolioError> {
        let mut names_array: Vec<Object> = Vec::new();
        let mut next_id = self.doc.max_id + 1;

        for (name, data) in &self.files {
            let stream_id = (next_id, 0u16);
            let spec_id = (next_id + 1, 0u16);
            next_id += 2;

            // EmbeddedFile stream
            let mut sd = Dictionary::new();
            sd.set("Type", "EmbeddedFile");
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
            sd.set("Params", Object::Dictionary(params));
            let mut stream = Stream::new(sd, data.clone());
            stream.compress().ok();
            self.doc.objects.insert(stream_id, Object::Stream(stream));

            // Filespec
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
            self.doc.objects.insert(spec_id, Object::Dictionary(fs));

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
        self.doc.objects.insert(ef_id, Object::Dictionary(ef_dict));

        // Update catalog: add /Names pointing to our EmbeddedFiles
        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Reference(ef_id));

        let catalog = self.doc.catalog_mut()?;
        catalog.set("Names", Object::Dictionary(names));

        // Update max_id
        self.doc.max_id = self.doc.objects.keys().map(|&(id, _)| id).max().unwrap_or(0);

        // Save
        let mut buf = Vec::new();
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }
}
