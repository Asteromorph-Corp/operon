operon_macros::use_psql_storage!(DataStorage);

#[tokio::main]
async fn main() {
    // Placeholder
    let var = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    println!("{var}");
}
