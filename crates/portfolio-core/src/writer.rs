use std::io::Cursor;

use lopdf::{Dictionary, Object, Stream};

use crate::error::PortfolioError;

pub struct PortfolioBuilder {
    doc: lopdf::Document,
    files: Vec<(String, Vec<u8>)>,
}

impl PortfolioBuilder {
    pub fn new() -> Result<Self, PortfolioError> {
        let template = include_bytes!("../../../samples/Portfolio2.pdf");
        let doc = lopdf::Document::load_from(Cursor::new(template.as_ref()))?;
        Ok(Self { doc, files: Vec::new() })
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
            self.doc.objects.insert(stream_id, Object::Stream(stream));

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
            self.doc.objects.insert(spec_id, Object::Dictionary(fs));

            names_array.push(Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
            names_array.push(Object::Reference(spec_id));
        }

        let mut ef_dict = Dictionary::new();
        ef_dict.set("Names", Object::Array(names_array));
        let ef_id = (next_id, 0u16);
        self.doc.objects.insert(ef_id, Object::Dictionary(ef_dict));

        let mut names = Dictionary::new();
        names.set("EmbeddedFiles", Object::Reference(ef_id));

        let catalog = self.doc.catalog_mut()?;
        catalog.set("Names", Object::Dictionary(names));

        self.doc.max_id = self.doc.objects.keys().map(|&(id, _)| id).max().unwrap_or(0);
        let mut buf = Vec::new();
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }
}
