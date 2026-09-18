use std::fs;
use std::path::PathBuf;

use quote::ToTokens;
use syn::parse_quote;

use crate::test_utils::normalize_string::normalize_string;

fn get_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn prettify_file(file: &syn::File) -> String {
    let prettified = prettyplease::unparse(file);
    normalize_string(&prettified)
}

fn prettify_item(item: &impl ToTokens) -> String {
    let file: syn::File = syn::parse2(item.to_token_stream()).expect("item didn’t parse");
    prettify_file(&file)
}

fn prettify_items_in_trait(items: &[impl ToTokens]) -> String {
    let file: syn::File = parse_quote! {
        pub trait DummyTrait {
            #(#items)*
        }
    };
    prettify_file(&file)
}

fn load_fixture(fixture_path: &str) -> String {
    let src = fs::read_to_string(get_fixture_path(fixture_path)).expect("fixture file missing");
    let normalized = normalize_string(&src);
    let file: syn::File = syn::parse_str(&normalized).expect("fixture didn’t parse");
    prettify_file(&file)
}

pub(crate) fn load_fixture_in_trait(fixture_path: &str) -> String {
    let src = fs::read_to_string(get_fixture_path(fixture_path)).expect("fixture file missing");
    let normalized = normalize_string(&src);
    let items: proc_macro2::TokenStream = normalized.parse().expect("fixture didn’t parse");
    let file: syn::File = parse_quote! {
        pub trait DummyTrait {
            #items
        }
    };
    prettify_file(&file)
}

pub(crate) fn assert_item_eq(item: &impl ToTokens, fixture_path: &str) {
    let pretty = prettify_item(item);
    let expected = load_fixture(fixture_path);
    pretty_assertions::assert_eq!(pretty, expected);
}

/// Test helper to compare a list of items in a trait context.
///
/// Since `prettyplease` can only format complete files, we wrap the items in a dummy trait.
pub(crate) fn assert_items_eq_in_trait(items: &[impl ToTokens], fixture_path: &str) {
    let pretty = prettify_items_in_trait(items);
    let expected = load_fixture_in_trait(fixture_path);
    pretty_assertions::assert_eq!(pretty, expected);
}
