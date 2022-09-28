#![feature(proc_macro_span, track_path)]

use std::env;

use include_preprocessor::{preprocess, SourceTracker, SearchPaths};
use proc_macro::tracked_path;
use proc_macro::{Literal, Span, TokenStream, TokenTree};
use std::path::Path;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn include_str_ipp(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);

    let span = Span::call_site();
    let source_path = span.source_file().path();
    let source_dir = source_path.parent().unwrap();

    let mut search_paths = SearchPaths::new();
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    search_paths.push_base_path(cargo_manifest_dir);

    let source_join = source_dir.join(path.value());

    let output = if source_join.is_file() {
        let buffer = String::new();

        preprocess(source_join, search_paths, buffer, &mut ProcMacroPathTracker).unwrap()
    } else {
        panic!("Entry (`{:?}`) point is not a file!", source_join);
    };

    let token = Literal::string(&output);

    let tree: TokenTree = token.into();

    tree.into()
}

struct ProcMacroPathTracker;

impl SourceTracker for ProcMacroPathTracker {
    fn track(&mut self, path: &Path, _source: &str) {
        tracked_path::path(path.to_str().expect("cannot track non-unicode path"));
    }
}
