//! An in-memory entity storage for this pipeline, to time against `PsqlCookingStorage`.
//!
//! The `mem` subcommand runs the pipeline on this storage, paired with the in-memory metadata
//! backend to take Postgres out of the run entirely.

use async_trait::async_trait;
use dashmap::DashMap;
use operon::error::StorageError;
use operon::{Entity, OperonStorage};

use crate::{A, B, C, CookingStorage, D, E, F};

/// An entity storage holding one `DashMap` per entity, keyed by the entity's coordinate.
///
/// `define_operon!` generates the `CookingStorage` trait, which asks for a `get`/`put` pair per
/// entity; the batched accessors it also declares come with default implementations built on those.
/// That trait plus [`OperonStorage`] is everything a storage backend has to provide.
#[derive(Default)]
pub struct DashMapCookingStorage {
    a: DashMap<[usize; 1], A>,
    b: DashMap<[usize; 2], B>,
    c: DashMap<[usize; 2], C>,
    d: DashMap<[usize; 3], D>,
    e: DashMap<[usize; 2], E>,
    f: DashMap<[usize; 1], F>,
}

#[async_trait]
impl OperonStorage for DashMapCookingStorage {
    async fn init(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), StorageError> {
        self.a.clear();
        self.b.clear();
        self.c.clear();
        self.d.clear();
        self.e.clear();
        self.f.clear();
        Ok(())
    }

    // The footprint operations stay at their defaults: they exist to resume a previous run, which
    // this storage cannot outlive.
}

#[async_trait]
impl CookingStorage for DashMapCookingStorage {
    async fn get_a(&self, coordinate: [usize; 1]) -> Result<Option<A>, StorageError> {
        Ok(self.a.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_a(&self, entity: Entity<1, A>) -> Result<(), StorageError> {
        self.a.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_b(&self, coordinate: [usize; 2]) -> Result<Option<B>, StorageError> {
        Ok(self.b.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_b(&self, entity: Entity<2, B>) -> Result<(), StorageError> {
        self.b.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_c(&self, coordinate: [usize; 2]) -> Result<Option<C>, StorageError> {
        Ok(self.c.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_c(&self, entity: Entity<2, C>) -> Result<(), StorageError> {
        self.c.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_d(&self, coordinate: [usize; 3]) -> Result<Option<D>, StorageError> {
        Ok(self.d.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_d(&self, entity: Entity<3, D>) -> Result<(), StorageError> {
        self.d.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_e(&self, coordinate: [usize; 2]) -> Result<Option<E>, StorageError> {
        Ok(self.e.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_e(&self, entity: Entity<2, E>) -> Result<(), StorageError> {
        self.e.insert(entity.coordinate, entity.value);
        Ok(())
    }

    async fn get_f(&self, coordinate: [usize; 1]) -> Result<Option<F>, StorageError> {
        Ok(self.f.get(&coordinate).map(|entry| entry.clone()))
    }

    async fn put_f(&self, entity: Entity<1, F>) -> Result<(), StorageError> {
        self.f.insert(entity.coordinate, entity.value);
        Ok(())
    }
}
