mod utils;

use diary_generator::Generator;
use std::fs;
use tempdir::TempDir;
use utils::DirEntry;

#[tokio::test]
async fn empty_index() {
    let cwd = TempDir::new("empty_index").unwrap();

    let generator = Generator::new(&cwd, Vec::new()).await.unwrap();
    generator
        .generate_index_page()
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        DirEntry::breakdown(&cwd),
        DirEntry::dir(
            cwd.path().file_name().unwrap(),
            [DirEntry::dir("output", [DirEntry::file("index.html")])]
        ),
    );

    assert_eq!(
        fs::read_to_string(cwd.path().join("output").join("index.html")).unwrap(),
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="description" content="A neat diary"><link rel="stylesheet" href="/katex/katex.min.css"><title>Diary</title><meta property="og:title" content="Diary"><meta property="og:description" content="A neat diary"><meta property="og:locale" content="en_US"></head><body><header></header><main></main><footer></footer></body></html>"#
    );
}
