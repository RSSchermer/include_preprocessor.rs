#![feature(proc_macro_span)]

use std::env;

use include_preprocessor::{preprocess, SearchPaths};
use proc_macro::{Literal, Span, TokenStream, TokenTree};
use proc_macro_hack::proc_macro_hack;
use syn::{parse_macro_input, LitStr};

#[proc_macro_hack]
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

        preprocess(source_join, search_paths, buffer).unwrap()
    } else {
        panic!("Entry (`{:?}`) point is not a file!", source_join);
    };

    let token = Literal::string(&output);

    let tree: TokenTree = token.into();

    tree.into()
}
