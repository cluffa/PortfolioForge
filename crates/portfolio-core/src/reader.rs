use std::io::Cursor;

use crate::error::PortfolioError;
use crate::types::{EmbeddedFile, PortfolioMetadata, ViewMode};

/// An opened PDF Portfolio with its embedded files extracted into memory
pub struct Portfolio {
    files: Vec<EmbeddedFile>,
    metadata: PortfolioMetadata,
}

impl Portfolio {
    /// Check if raw PDF data is a Portfolio (has /Collection in catalog)
    pub fn is_portfolio(data: &[u8]) -> bool {
        lopdf::Document::load_from(Cursor::new(data))
            .ok()
            .and_then(|doc| {
                doc.catalog()
                    .ok()
                    .and_then(|cat| cat.get(b"Collection").ok().cloned())
            })
            .is_some()
    }

    /// Open a PDF Portfolio and extract its embedded files into memory
    pub fn open(data: &[u8]) -> Result<Self, PortfolioError> {
        let doc = lopdf::Document::load_from(Cursor::new(data))?;

        // Check it's a portfolio
        let catalog = doc.catalog()?;
        let collection = match catalog.get_deref(b"Collection", &doc) {
            Ok(coll) => coll,
            Err(_) => return Err(PortfolioError::NotAPortfolio),
        };

        let pdf_version = doc.version.clone();

        // Determine view mode
        let view_mode = collection
            .as_dict()
            .ok()
            .and_then(|coll_dict| coll_dict.get(b"View").ok())
            .and_then(|v| v.as_name().ok())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|v| match v {
                "D" => ViewMode::Details,
                "T" => ViewMode::Tile,
                "H" => ViewMode::Hidden,
                _ => ViewMode::Details,
            });

        // Extract embedded files from name tree
        let files = Self::extract_files(&doc)?;
        let file_count = files.len();

        Ok(Self {
            files,
            metadata: PortfolioMetadata {
                pdf_version,
                view_mode,
                file_count,
            },
        })
    }

    pub fn files(&self) -> &[EmbeddedFile] {
        &self.files
    }

    pub fn metadata(&self) -> &PortfolioMetadata {
        &self.metadata
    }

    pub fn extract_all(
        &self,
        output_dir: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, PortfolioError> {
        std::fs::create_dir_all(output_dir)?;
        let mut paths = Vec::new();
        for file in &self.files {
            let path = output_dir.join(&file.name);
            std::fs::write(&path, &file.data)?;
            paths.push(path);
        }
        Ok(paths)
    }

    pub fn extract_file(
        &self,
        name: &str,
        output_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, PortfolioError> {
        let file = self
            .files
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| PortfolioError::FileNotFound(name.to_string()))?;
        std::fs::create_dir_all(output_dir)?;
        let path = output_dir.join(&file.name);
        std::fs::write(&path, &file.data)?;
        Ok(path)
    }

    /// Walk the /EmbeddedFiles name tree and extract all file entries
    fn extract_files(doc: &lopdf::Document) -> Result<Vec<EmbeddedFile>, PortfolioError> {
        let mut files = Vec::new();

        let catalog = doc.catalog()?;

        // Catalog -> /Names (dereference)
        let names_dict = match catalog.get_deref(b"Names", doc) {
            Ok(names) => names,
            Err(_) => return Ok(files),
        };

        let names_dict = names_dict
            .as_dict()
            .map_err(|_| PortfolioError::ParseError(lopdf::Error::ObjectType {
                expected: "Dictionary",
                found: "other",
            }))?;

        // /Names -> /EmbeddedFiles (dereference)
        let ef_obj = match names_dict.get_deref(b"EmbeddedFiles", doc) {
            Ok(ef) => ef,
            Err(_) => return Ok(files),
        };

        let ef_dict = ef_obj
            .as_dict()
            .map_err(|_| PortfolioError::ParseError(lopdf::Error::ObjectType {
                expected: "Dictionary",
                found: "other",
            }))?;

        // /EmbeddedFiles -> /Names array (dereference)
        let names_array = match ef_dict.get_deref(b"Names", doc) {
            Ok(arr) => arr,
            Err(_) => {
                if ef_dict.get(b"Kids").is_ok() {
                    return Ok(files);
                }
                return Ok(files);
            }
        };

        let names = names_array
            .as_array()
            .map_err(|_| PortfolioError::ParseError(lopdf::Error::ObjectType {
                expected: "Array",
                found: "other",
            }))?;

        // Parse the alternating [ name ref name ref ... ] array
        let mut i = 0;
        while i + 1 < names.len() {
            // The name is a PDF string (Object::String)
            let name_bytes = names[i]
                .as_str()
                .unwrap_or(b"unknown");
            let name = String::from_utf8_lossy(name_bytes).into_owned();

            let file_spec_ref = &names[i + 1];
            let file_spec_id = file_spec_ref.as_reference().ok();

            if let Some(id) = file_spec_id {
                if let Ok(file_spec) = doc.get_object(id) {
                    if let Ok(file_dict) = file_spec.as_dict() {
                        // Get filename from /UF or /F
                        let raw_name = file_dict
                            .get(b"UF")
                            .or_else(|_| file_dict.get(b"F"));

                        let filename = if let Ok(n) = raw_name {
                            String::from_utf8_lossy(n.as_str().unwrap_or(b""))
                                .into_owned()
                        } else {
                            name.clone()
                        };

                        // Get embedded file data, decompressing if needed
                        let data = file_dict
                            .get(b"EF")
                            .ok()
                            .and_then(|ef| ef.as_dict().ok())
                            .and_then(|ef_dict| ef_dict.get(b"F").ok())
                            .and_then(|stream_ref| {
                                doc.get_object(stream_ref.as_reference().ok()?).ok()
                            })
                            .and_then(|obj| {
                                obj.as_stream().ok().map(|s| {
                                    s.decompressed_content().unwrap_or_else(|_| s.content.clone())
                                })
                            })
                            .unwrap_or_default();

                        let size = data.len() as u64;
                        let mime_type = guess_mime(&filename);

                        files.push(EmbeddedFile {
                            name: filename,
                            size,
                            mime_type,
                            data,
                        });
                    }
                }
            }
            i += 2;
        }

        Ok(files)
    }
}

fn guess_mime(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "docx" | "doc" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "xlsx" | "xls" => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        "pptx" | "ppt" => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    }
    .to_string()
}
