// # Operon Example 5: Fully in-memory
//
// Runs a pipeline with no database at all: the metadata goes to the in-memory backend, selected
// with `MetaBackendOptions::mem()`, and the entity data to the `DashMapStorage` below. Nothing
// survives the process. See ex1 for an introduction to Operon itself.

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use operon::error::{StorageError, UserError};
use operon::options::{MetaBackendOptions, OperonOptions, UiMode};
use operon::{Entity, Operon, OperonService, OperonStorage, define_operon};

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
    async fn alpha(&self) -> Result<Vec<A>, UserError> {
        Ok(vec![
            "the quick brown fox".to_owned(),
            "jumps over the lazy dog".to_owned(),
        ])
    }

    async fn beta(&self, document: A) -> Result<Vec<B>, UserError> {
        Ok(document.split_whitespace().map(str::to_owned).collect())
    }

    async fn gamma(&self, document: A) -> Result<Vec<C>, UserError> {
        let mut characters = document
            .chars()
            .filter(|character| !character.is_whitespace())
            .map(String::from)
            .collect::<Vec<_>>();
        characters.sort();
        characters.dedup();
        Ok(characters)
    }

    async fn delta(&self, word: B, character: C) -> Result<D, UserError> {
        Ok(word.matches(&character).count())
    }
}

/// An entity storage holding one `DashMap` per entity, keyed by the entity's coordinate.
///
/// `define_operon!` generates the `FanoutStorage` trait, which asks for a `get`/`put` pair per
/// entity; the batched accessors it also declares come with default implementations built on those.
/// That trait plus [`OperonStorage`] is everything a storage backend has to provide.
#[derive(Default)]
struct DashMapStorage {
    documents: DashMap<[usize; 1], A>,
    words: DashMap<[usize; 2], B>,
    characters: DashMap<[usize; 2], C>,
    counts: DashMap<[usize; 3], D>,
}

#[async_trait]
impl OperonStorage for DashMapStorage {
    async fn init(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), StorageError> {
        self.documents.clear();
        self.words.clear();
        self.characters.clear();
        self.counts.clear();
        Ok(())
    }

    // The footprint operations stay at their defaults: they exist to resume a previous run, which
    // this storage cannot outlive.
}

#[async_trait]
impl FanoutStorage for DashMapStorage {
    async fn get_a(&self, coordinate: [usize; 1]) -> Result<Option<A>, StorageError> {
        Ok(self.documents.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_a(&self, entity: Entity<1, A>) -> Result<(), StorageError> {
        self.documents.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_b(&self, coordinate: [usize; 2]) -> Result<Option<B>, StorageError> {
        Ok(self.words.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_b(&self, entity: Entity<2, B>) -> Result<(), StorageError> {
        self.words.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_c(&self, coordinate: [usize; 2]) -> Result<Option<C>, StorageError> {
        Ok(self.characters.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_c(&self, entity: Entity<2, C>) -> Result<(), StorageError> {
        self.characters.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_d(&self, coordinate: [usize; 3]) -> Result<Option<D>, StorageError> {
        Ok(self.counts.get(&coordinate).map(|entry| *entry))
    }

    async fn put_d(&self, entity: Entity<3, D>) -> Result<(), StorageError> {
        self.counts.insert(entity.coordinate, entity.value);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Headless mode starts the run immediately and exits once it finishes, so this example needs
    // neither a database nor a terminal to drive it.
    let operon_options =
        OperonOptions::from_backend(MetaBackendOptions::mem()).with_ui_mode(UiMode::Headless);

    let storage = Arc::new(DashMapStorage::default());
    let operon: Operon<WordCounter, DashMapStorage> =
        Operon::new(WordCounter, storage.clone(), operon_options);

    operon.run().await?;

    println!("Counted over {} documents.", storage.documents.len());
    for document in storage.documents.iter() {
        let [i] = *document.key();
        println!("\n{:?}", document.value());

        for word in storage.words.iter().filter(|word| word.key()[0] == i) {
            let [_, j] = *word.key();
            let occurrences = storage
                .characters
                .iter()
                .filter(|character| character.key()[0] == i)
                .filter_map(|character| {
                    let [_, k] = *character.key();
                    let count = *storage.counts.get(&[i, j, k])?;
                    (count > 0).then(|| format!("{}x{}", character.value(), count))
                })
                .collect::<Vec<_>>();
            println!("  {:<6} {}", word.value(), occurrences.join(" "));
        }
    }

    Ok(())
}
