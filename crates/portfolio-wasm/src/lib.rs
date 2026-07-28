use wasm_bindgen::prelude::*;

/// Create a PDF Portfolio from files. `files_json` is JSON array of {name, data: byte array}.
#[wasm_bindgen]
pub fn create_portfolio(files_json: &str) -> Result<Vec<u8>, JsValue> {
    let files: Vec<FileEntry> = serde_json::from_str(files_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut builder = portfolio_core::PortfolioBuilder::new()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    for file in &files {
        builder.add_file(&file.name, file.data.clone(), "");
    }

    builder
        .build()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// List files in a PDF Portfolio. Returns JSON array of {name, size, mime_type}.
#[wasm_bindgen]
pub fn list_portfolio(data: Vec<u8>) -> Result<String, JsValue> {
    let pf = portfolio_core::Portfolio::open(&data)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let files: Vec<FileInfo> = pf
        .files()
        .iter()
        .map(|f| FileInfo {
            name: f.name.clone(),
            size: f.size,
            mime_type: f.mime_type.clone(),
        })
        .collect();
    serde_json::to_string(&files).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract a file from a portfolio by name. Returns file bytes.
#[wasm_bindgen]
pub fn extract_file(portfolio_data: Vec<u8>, name: &str) -> Result<Vec<u8>, JsValue> {
    let pf = portfolio_core::Portfolio::open(&portfolio_data)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    for file in pf.files() {
        if file.name == name {
            return Ok(file.data.clone());
        }
    }
    Err(JsValue::from_str("File not found"))
}

/// Validate a PDF Portfolio. Returns JSON array of issue strings.
#[wasm_bindgen]
pub fn validate_portfolio(data: Vec<u8>) -> Result<String, JsValue> {
    let issues = portfolio_core::validate(&data)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&issues).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Check if data is a PDF Portfolio.
#[wasm_bindgen]
pub fn is_portfolio(data: Vec<u8>) -> bool {
    portfolio_core::Portfolio::is_portfolio(&data)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FileEntry {
    name: String,
    data: Vec<u8>,
}

#[derive(serde::Serialize)]
struct FileInfo {
    name: String,
    size: u64,
    mime_type: String,
}
