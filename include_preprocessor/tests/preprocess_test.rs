use std::env;
use std::path::Path;

use include_preprocessor::{preprocess, SourceTracker, SearchPaths};
use std::collections::HashSet;

struct TestPathTracker {
    paths: HashSet<String>,
}

impl TestPathTracker {
    fn new() -> Self {
        TestPathTracker {
            paths: HashSet::new(),
        }
    }
}

impl SourceTracker for TestPathTracker {
    fn track(&mut self, path: &Path, _source: &str) {
        self.paths.insert(path.to_str().unwrap().to_string());
    }
}

#[test]
fn test_preprocess_valid() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut search_paths = SearchPaths::new();

    search_paths.push_base_path(&cargo_manifest_dir);

    let base_path: &Path = cargo_manifest_dir.as_ref();
    let entry_point = base_path.join("tests/valid/a.txt");
    let buffer = String::new();
    let mut path_tracker = TestPathTracker::new();
    let res = preprocess(entry_point, search_paths, buffer, &mut path_tracker);

    assert!(res.is_ok());

    let actual = res.unwrap();
    let expected = include_str!("expected.txt");

    assert_eq!(&actual, expected);

    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid/a.txt").to_str().unwrap()));
    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid/b.txt").to_str().unwrap()));
    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid/c.txt").to_str().unwrap()));
}

#[test]
fn test_preprocess_valid_2() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut search_paths = SearchPaths::new();

    search_paths.push_base_path(&cargo_manifest_dir);

    let base_path: &Path = cargo_manifest_dir.as_ref();
    let entry_point = base_path.join("tests/valid_2/a.txt");
    let buffer = String::new();
    let mut path_tracker = TestPathTracker::new();
    let res = preprocess(entry_point, search_paths, buffer, &mut path_tracker);

    assert!(res.is_ok());

    let actual = res.unwrap();
    let expected = include_str!("expected_2.txt");

    assert_eq!(&actual, expected);

    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid_2/a.txt").to_str().unwrap()));
    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid_2/b.txt").to_str().unwrap()));
    assert!(path_tracker
        .paths
        .contains(base_path.join("tests/valid_2/c.txt").to_str().unwrap()));
}
