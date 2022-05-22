use std::env;
use std::path::Path;

use include_preprocessor::{preprocess, SearchPaths};

#[test]
fn test_preprocess_valid() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut search_paths = SearchPaths::new();

    search_paths.push_base_path(&cargo_manifest_dir);

    let base_path: &Path = cargo_manifest_dir.as_ref();
    let entry_point = base_path.join("tests/valid/a.txt");
    let buffer = String::new();
    let res = preprocess(entry_point, search_paths, buffer);

    assert!(res.is_ok());

    let actual = res.unwrap();
    let expected = include_str!("expected.txt");

    assert_eq!(&actual, expected);
}

#[test]
fn test_preprocess_valid_2() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut search_paths = SearchPaths::new();

    search_paths.push_base_path(&cargo_manifest_dir);

    let base_path: &Path = cargo_manifest_dir.as_ref();
    let entry_point = base_path.join("tests/valid_2/a.txt");
    let buffer = String::new();
    let res = preprocess(entry_point, search_paths, buffer);

    assert!(res.is_ok());

    let actual = res.unwrap();
    let expected = include_str!("expected_2.txt");

    assert_eq!(&actual, expected);
}
