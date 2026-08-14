//! Compiles a pipeline under names other than `operon`.

use my_operon::options::{PsqlMetaStorageOptions, PsqlStorageOptions};
use my_operon::{Operon, OperonService, define_operon};

type A = ();
type B = ();

define_operon! {
    simple = {
        A<i> = alpha();
        B = beta(A) for i;
    }
}

/// `proc-macro-crate` should resolve `#operon` to `my_operon` even without
/// `#[operon(crate = "my_operon")]`.
#[derive(OperonService)]
pub struct MyOperonService;

#[async_trait::async_trait]
impl SimpleService for MyOperonService {
    async fn alpha(&self) -> Result<Vec<A>, Self::Error> {
        Ok(vec![()])
    }

    async fn beta(&self, _a: A) -> Result<B, Self::Error> {
        Ok(())
    }
}
impl MyOperonService {
    pub fn build(
        database_uri: &str,
    ) -> Result<
        Operon<Self, PsqlSimpleStorage, my_operon::PsqlMetaStorage>,
        Box<dyn std::error::Error>,
    > {
        let storage = PsqlStorageOptions::new(database_uri).build::<PsqlSimpleStorage>()?;
        let meta = PsqlMetaStorageOptions::new(database_uri).build()?;

        Ok(Operon::new(Self, storage, meta))
    }
}

/// An alias for testing "crate::" paths.
pub mod aliased {
    pub use my_operon::*;
}

#[derive(OperonService)]
#[operon(crate = "crate::aliased")]
pub struct AliasedService;

#[async_trait::async_trait]
impl SimpleService for AliasedService {
    async fn alpha(&self) -> Result<Vec<A>, Self::Error> {
        Ok(vec![()])
    }

    async fn beta(&self, _a: A) -> Result<B, Self::Error> {
        Ok(())
    }
}
impl AliasedService {
    pub fn build(
        database_uri: &str,
    ) -> Result<
        Operon<Self, PsqlSimpleStorage, my_operon::PsqlMetaStorage>,
        Box<dyn std::error::Error>,
    > {
        let storage = PsqlStorageOptions::new(database_uri).build::<PsqlSimpleStorage>()?;
        let meta = PsqlMetaStorageOptions::new(database_uri).build()?;

        Ok(Operon::new(Self, storage, meta))
    }
}

/// Reaches the pipeline entry points generated under a renamed `operon`.
#[test]
fn pipelines_build_under_a_renamed_crate() {
    let _ = (MyOperonService::build, AliasedService::build);
}
