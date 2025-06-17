use operon_macros::*;

use_psql_storage!();

#[tokio::main]
async fn main() {
    println!("Hello, Operon!");
}
