async fn get_all_b_j(&self, [i]: [usize; 1usize]) -> Result<Vec<B>, operon::storage::StorageError> {
    let entities = self
        .conn()
        .await?
        .entity(self.entities_meta.b)
        .batch_get([i], ["j"])
        .await?;

    let mut result: Vec<B> = Default::default();
    for entity in entities {
        result.push(entity);
    }

    Ok(result)
}

async fn get_all_c_k(&self, [i]: [usize; 1usize]) -> Result<Vec<C>, operon::storage::StorageError> {
    let entities = self
        .conn()
        .await?
        .entity(self.entities_meta.c)
        .batch_get([i], ["k"])
        .await?;

    let mut result: Vec<C> = Default::default();
    for entity in entities {
        result.push(entity);
    }

    Ok(result)
}

async fn get_all_d_j(
    &self,
    [i, k]: [usize; 2usize],
) -> Result<Vec<D>, operon::storage::StorageError> {
    let entities = self
        .conn()
        .await?
        .entity(self.entities_meta.d)
        .batch_get([i, k], ["j"])
        .await?;

    let mut result: Vec<D> = Default::default();
    for entity in entities {
        result.push(entity);
    }

    Ok(result)
}

async fn get_all_e_k(&self, [i]: [usize; 1usize]) -> Result<Vec<E>, operon::storage::StorageError> {
    let entities = self
        .conn()
        .await?
        .entity(self.entities_meta.e)
        .batch_get([i], ["k"])
        .await?;

    let mut result: Vec<E> = Default::default();
    for entity in entities {
        result.push(entity);
    }
    Ok(result)
}
