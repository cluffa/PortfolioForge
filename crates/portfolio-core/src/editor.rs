use std::io::Cursor;

use lopdf::{Dictionary, Object, ObjectId, Stream};

use crate::error::PortfolioError;

/// Edit an existing PDF Portfolio: add, remove, replace, reorder, rename files
pub struct PortfolioEditor {
    doc: lopdf::Document,
}

impl PortfolioEditor {
    /// Open an existing portfolio for editing
    pub fn open(data: &[u8]) -> Result<Self, PortfolioError> {
        let doc = lopdf::Document::load_from(Cursor::new(data))?;
        let catalog = doc.catalog()?;
        if catalog.get(b"Collection").is_err() {
            return Err(PortfolioError::NotAPortfolio);
        }
        Ok(Self { doc })
    }

    /// Add a file to the portfolio. If a file with the same name exists, it is replaced.
    pub fn add_file(&mut self, name: &str, data: Vec<u8>) -> Result<&mut Self, PortfolioError> {
        let next_id = self.doc.max_id + 1;
        let file_spec_id = (next_id, 0u16);
        let stream_id = (next_id + 1, 0u16);
        self.doc.max_id = next_id + 1;

        // Embedded file stream
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
        let mut stream = Stream::new(stream_dict, data);
        stream.allows_compression = false;
        self.doc.objects.insert(stream_id, Object::Stream(stream));

        // File spec
        let mut fs_dict = Dictionary::new();
        fs_dict.set("Type", "Filespec");
        fs_dict.set(
            "F",
            Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        fs_dict.set(
            "UF",
            Object::String(name.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        let mut ef = Dictionary::new();
        ef.set("F", Object::Reference(stream_id));
        fs_dict.set("EF", Object::Dictionary(ef));
        self.doc
            .objects
            .insert(file_spec_id, Object::Dictionary(fs_dict));

        // Add to the /EmbeddedFiles name tree
        self.add_to_names_tree(name, file_spec_id)?;
        Ok(self)
    }

    /// Remove a file from the portfolio by name
    pub fn remove_file(&mut self, name: &str) -> Result<&mut Self, PortfolioError> {
        self.remove_from_names_tree(name)?;
        Ok(self)
    }

    /// Replace a file (remove old + add new). If old doesn't exist, just adds.
    pub fn replace_file(
        &mut self,
        old_name: &str,
        new_name: &str,
        data: Vec<u8>,
    ) -> Result<&mut Self, PortfolioError> {
        let _ = self.remove_file(old_name);
        self.add_file(new_name, data)?;
        Ok(self)
    }

    /// Rename a file
    pub fn rename_file(
        &mut self,
        old_name: &str,
        new_name: &str,
    ) -> Result<&mut Self, PortfolioError> {
        let array = self.get_names_array_mut()?;
        if let Some(array) = array {
            for i in (0..array.len()).step_by(2) {
                let current = array[i]
                    .as_str()
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_default();
                if current == old_name {
                    array[i] = Object::String(
                        new_name.as_bytes().to_vec(),
                        lopdf::StringFormat::Literal,
                    );
                    // Also update /F and /UF in the file spec
                    let ref_id = array[i + 1].as_reference().ok();
                    if let Some(id) = ref_id {
                        if let Ok(obj) = self.doc.get_object_mut(id) {
                            if let Ok(dict) = obj.as_dict_mut() {
                                dict.set(
                                    "F",
                                    Object::String(
                                        new_name.as_bytes().to_vec(),
                                        lopdf::StringFormat::Literal,
                                    ),
                                );
                                dict.set(
                                    "UF",
                                    Object::String(
                                        new_name.as_bytes().to_vec(),
                                        lopdf::StringFormat::Literal,
                                    ),
                                );
                            }
                        }
                    }
                    break;
                }
            }
        }
        Ok(self)
    }

    /// Reorder files to match the given order of names. Unlisted names stay at the end.
    pub fn reorder_files(&mut self, ordered_names: &[&str]) -> Result<&mut Self, PortfolioError> {
        let array = self.get_names_array_mut()?;
        if let Some(array) = array {
            let mut entries: Vec<(String, Object)> = Vec::new();
            let mut i = 0;
            while i + 1 < array.len() {
                let name = array[i]
                    .as_str()
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_default();
                let ref_obj = array[i + 1].clone();
                entries.push((name, ref_obj));
                i += 2;
            }

            let mut new_array: Vec<Object> = Vec::new();
            for target in ordered_names {
                if let Some(pos) = entries.iter().position(|(n, _)| n == *target) {
                    let (name, ref_obj) = entries.remove(pos);
                    new_array.push(Object::String(
                        name.into_bytes(),
                        lopdf::StringFormat::Literal,
                    ));
                    new_array.push(ref_obj);
                }
            }
            for (name, ref_obj) in entries {
                new_array.push(Object::String(
                    name.into_bytes(),
                    lopdf::StringFormat::Literal,
                ));
                new_array.push(ref_obj);
            }
            *array = new_array;
        }
        Ok(self)
    }

    /// Save the modified portfolio to bytes
    pub fn save(mut self) -> Result<Vec<u8>, PortfolioError> {
        let max_id = self.doc.objects.keys().map(|&(id, _)| id).max().unwrap_or(0);
        self.doc.max_id = max_id;
        let mut buf = Vec::new();
        self.doc.save_to(&mut buf)?;
        Ok(buf)
    }

    // ── internal helpers ──

    /// Get mutable reference to the /Names array in /EmbeddedFiles
    fn get_names_array_mut(&mut self) -> Result<Option<&mut Vec<Object>>, PortfolioError> {
        // Ensure /Names exists
        let needs_create = {
            let catalog = self.doc.catalog()?;
            catalog.get(b"Names").is_err()
        };
        
        if needs_create {
            let ef_id = (self.doc.max_id + 1, 0u16);
            self.doc.max_id += 1;
            let mut ef = Dictionary::new();
            ef.set("Names", Object::Array(Vec::new()));
            self.doc.objects.insert(ef_id, Object::Dictionary(ef));
            let mut names = Dictionary::new();
            names.set("EmbeddedFiles", Object::Reference(ef_id));
            let catalog = self.doc.catalog_mut()?;
            catalog.set("Names", Object::Dictionary(names));
            return Ok(Some(
                self.doc.get_object_mut(ef_id)?.as_dict_mut()?.get_mut(b"Names")?.as_array_mut()?
            ));
        }

        // Find the EmbeddedFiles — always follow references to get its dict
        let ef_id = {
            let catalog = self.doc.catalog()?;
            let names_raw = catalog.get(b"Names")?;
            // names_raw may be inline dict or reference to a dict
            let names_dict = {
                if let Ok(names_id) = names_raw.as_reference() {
                    self.doc.get_object(names_id)?.as_dict()?
                } else {
                    names_raw.as_dict()?
                }
            };
            let ef_raw = names_dict.get(b"EmbeddedFiles")?;
            // ef_raw should be a reference in our case
            ef_raw.as_reference().map_err(|_| {
                PortfolioError::ParseError(lopdf::Error::ObjectType {
                    expected: "Reference",
                    found: "inline dict",
                })
            })?
        };

        let ef_dict = self.doc.get_object_mut(ef_id)?.as_dict_mut()?;
        Ok(Some(ef_dict.get_mut(b"Names")?.as_array_mut()?))
    }

    fn add_to_names_tree(&mut self, name: &str, file_spec_id: ObjectId) -> Result<(), PortfolioError> {
        let array = self.get_names_array_mut()?;
        if let Some(array) = array {
            // Remove existing entry with the same name
            let mut i = 0;
            while i + 1 < array.len() {
                let current = array[i]
                    .as_str()
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_default();
                if current == name {
                    array.remove(i); // name
                    array.remove(i); // reference
                } else {
                    i += 2;
                }
            }
            array.push(Object::String(
                name.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            ));
            array.push(Object::Reference(file_spec_id));
        }
        Ok(())
    }

    fn remove_from_names_tree(&mut self, name: &str) -> Result<(), PortfolioError> {
        let array = self.get_names_array_mut()?;
        if let Some(array) = array {
            let mut i = 0;
            while i + 1 < array.len() {
                let current = array[i]
                    .as_str()
                    .map(|b| String::from_utf8_lossy(b).to_string())
                    .unwrap_or_default();
                if current == name {
                    array.remove(i);
                    array.remove(i);
                } else {
                    i += 2;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Portfolio, PortfolioBuilder};

    #[test]
    fn test_add_to_existing() {
        let mut b = PortfolioBuilder::new().unwrap();
        b.add_file("a.pdf", b"AAA".to_vec(), "application/pdf");
        let pdf = b.build().unwrap();

        let mut editor = PortfolioEditor::open(&pdf).unwrap();
        editor.add_file("b.txt", b"BBB".to_vec()).unwrap();
        let modified = editor.save().unwrap();

        let pf = Portfolio::open(&modified).unwrap();
        let names: Vec<_> = pf.files().iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"a.pdf".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
    }

    #[test]
    fn test_remove_file() {
        let mut b = PortfolioBuilder::new().unwrap();
        b.add_file("a.pdf", b"AAA".to_vec(), "application/pdf");
        b.add_file("b.txt", b"BBB".to_vec(), "text/plain");
        let pdf = b.build().unwrap();

        let mut editor = PortfolioEditor::open(&pdf).unwrap();
        editor.remove_file("a.pdf").unwrap();
        let modified = editor.save().unwrap();

        let pf = Portfolio::open(&modified).unwrap();
        assert_eq!(pf.files().len(), 1);
        assert_eq!(pf.files()[0].name, "b.txt");
    }

    #[test]
    fn test_rename_file() {
        let mut b = PortfolioBuilder::new().unwrap();
        b.add_file("old.txt", b"data".to_vec(), "text/plain");
        let pdf = b.build().unwrap();

        let mut editor = PortfolioEditor::open(&pdf).unwrap();
        editor.rename_file("old.txt", "new.txt").unwrap();
        let modified = editor.save().unwrap();

        let pf = Portfolio::open(&modified).unwrap();
        assert_eq!(pf.files()[0].name, "new.txt");
    }

    #[test]
    fn test_reorder_files() {
        let mut b = PortfolioBuilder::new().unwrap();
        b.add_file("c.pdf", b"C".to_vec(), "application/pdf");
        b.add_file("a.pdf", b"A".to_vec(), "application/pdf");
        b.add_file("b.pdf", b"B".to_vec(), "application/pdf");
        let pdf = b.build().unwrap();

        let mut editor = PortfolioEditor::open(&pdf).unwrap();
        editor.reorder_files(&["b.pdf", "a.pdf", "c.pdf"]).unwrap();
        let modified = editor.save().unwrap();

        let pf = Portfolio::open(&modified).unwrap();
        let names: Vec<_> = pf.files().iter().map(|f| f.name.clone()).collect();
        assert_eq!(names, vec!["b.pdf", "a.pdf", "c.pdf"]);
    }

    #[test]
    fn test_replace_file() {
        let mut b = PortfolioBuilder::new().unwrap();
        b.add_file("old.txt", b"old".to_vec(), "text/plain");
        let pdf = b.build().unwrap();

        let mut editor = PortfolioEditor::open(&pdf).unwrap();
        editor.replace_file("old.txt", "new.txt", b"new!".to_vec()).unwrap();
        let modified = editor.save().unwrap();

        let pf = Portfolio::open(&modified).unwrap();
        assert_eq!(pf.files().len(), 1);
        assert_eq!(pf.files()[0].name, "new.txt");
        assert_eq!(pf.files()[0].data, b"new!");
    }
}
