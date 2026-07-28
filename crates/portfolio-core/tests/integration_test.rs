use portfolio_core::{Portfolio, PortfolioBuilder};

#[test]
fn test_detect_portfolio1() {
    let data = include_bytes!("../../../samples/Portfolio1.pdf");
    assert!(
        Portfolio::is_portfolio(data),
        "Portfolio1 should be detected as a portfolio"
    );
}

#[test]
fn test_open_portfolio1() {
    let data = include_bytes!("../../../samples/Portfolio1.pdf");
    let pf = Portfolio::open(data).expect("Should open Portfolio1");
    let files = pf.files();
    println!("Found {} files:", files.len());
    for f in files {
        println!("  {} ({} bytes, type: {})", f.name, f.size, f.mime_type);
    }
    assert!(!files.is_empty(), "Portfolio1 should contain files");
}

#[test]
fn test_extract_portfolio1() {
    let data = include_bytes!("../../../samples/Portfolio1.pdf");
    let pf = Portfolio::open(data).unwrap();
    let tmp = std::env::temp_dir().join("pf_test_extract_p1");
    let _ = std::fs::remove_dir_all(&tmp);
    let paths = pf.extract_all(&tmp).unwrap();
    assert!(!paths.is_empty(), "Should extract files from Portfolio1");
    for p in &paths {
        assert!(p.exists(), "Extracted file should exist: {:?}", p);
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_create_and_read_roundtrip() {
    let mut builder = PortfolioBuilder::new().expect("Should create builder");
    builder.add_file("hello.txt", b"Hello, Portfolio!".to_vec(), "text/plain");
    let pdf_data = builder.build().expect("Should build portfolio");

    // Verify it's detected as a portfolio
    assert!(
        Portfolio::is_portfolio(&pdf_data),
        "Created portfolio should be detected"
    );

    // Open it and verify contents
    let pf = Portfolio::open(&pdf_data).expect("Should open created portfolio");
    let files = pf.files();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "hello.txt");
    assert_eq!(files[0].data, b"Hello, Portfolio!");
}
