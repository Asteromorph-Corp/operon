// # Operon Example 5: Fully in-memory
//
// Runs a pipeline with no database at all: the metadata goes to the in-memory backend, selected
// with `MetaBackendOptions::mem()`, and the entity data to the in-memory storage. Nothing
// survives the process. See ex1 for an introduction to Operon itself.

use std::sync::Arc;

use async_trait::async_trait;
use operon::options::{MemMetaStorageOptions, MemStorageOptions, OperonOptions, UiMode};
use operon::{MemMetaStorage, Operon, OperonService, define_operon};

type A = String;
type B = String;
type C = String;
type D = usize;

define_operon! {
    fanout = {
        A<i> = alpha();
        B<j> = beta(A) for i;
        C<k> = gamma(A) for i;
        D = delta(B, C) for i, j, k;
    }
}

#[derive(OperonService)]
struct WordCounter;

#[async_trait]
impl FanoutService for WordCounter {
    async fn alpha(&self) -> Result<Vec<A>, Self::Error> {
        Ok(vec![
            "the quick brown fox".to_owned(),
            "jumps over the lazy dog".to_owned(),
        ])
    }

    async fn beta(&self, document: A) -> Result<Vec<B>, Self::Error> {
        Ok(document.split_whitespace().map(str::to_owned).collect())
    }

    async fn gamma(&self, document: A) -> Result<Vec<C>, Self::Error> {
        let mut characters = document
            .chars()
            .filter(|character| !character.is_whitespace())
            .map(String::from)
            .collect::<Vec<_>>();
        characters.sort();
        characters.dedup();
        Ok(characters)
    }

    async fn delta(&self, word: B, character: C) -> Result<D, Self::Error> {
        Ok(word.matches(&character).count())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Headless mode starts the run immediately and exits once it finishes, so this example needs
    // neither a database nor a terminal to drive it.
    let operon_options = OperonOptions::new().with_ui_mode(UiMode::Headless);

    let meta = MemMetaStorageOptions::new().build();
    let storage = Arc::new(MemStorageOptions::new().build());
    let operon: Operon<WordCounter, MemFanoutStorage, MemMetaStorage> =
        Operon::new(WordCounter, storage.clone(), meta).with_options(operon_options);

    operon.run().await?;

    println!("Counted over {} documents.", storage.a.len());
    for document in &storage.a {
        let [i] = *document.key();
        println!("\n{:?}", document.value());

        for word in storage.b.iter().filter(|word| word.key()[0] == i) {
            let [_, j] = *word.key();
            let occurrences = storage
                .c
                .iter()
                .filter(|character| character.key()[0] == i)
                .filter_map(|character| {
                    let [_, k] = *character.key();
                    let count = *storage.d.get(&[i, j, k])?;
                    (count > 0).then(|| format!("{}x{}", character.value(), count))
                })
                .collect::<Vec<_>>();
            println!("  {:<6} {}", word.value(), occurrences.join(" "));
        }
    }

    Ok(())
}
