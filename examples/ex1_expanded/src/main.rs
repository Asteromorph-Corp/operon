use async_trait::async_trait;
use dashmap::DashMap;
use ex1_expanded::operon::{dimension::*, entity::*, *};
use futures::SinkExt;
use rand::Rng;
use std::sync::Arc;

// Example in-memory storage that does not persist data.
// This is here for demonstration purposes only.
#[derive(Debug)]
#[allow(unused, dead_code)]
struct InMemoryStorage {
    a_storage: DashMap<I, A>,
    b_storage: DashMap<(I, J), B>,
    c_storage: DashMap<(I, K), C>,
    d_storage: DashMap<(I, J, K), D>,
    e_storage: DashMap<(I, K), E>,
    f_storage: DashMap<I, F>,
}
// Implement the OperonStorage trait
#[async_trait]
#[allow(unused, dead_code, clippy::clone_on_copy)]
impl OperonStorage for InMemoryStorage {
    async fn clear(&self) -> anyhow::Result<()> {
        self.b_storage.clear();
        self.c_storage.clear();
        self.d_storage.clear();
        self.e_storage.clear();
        self.f_storage.clear();
        Ok(())
    }

    async fn put_a(&self, i: I, value: &A) -> anyhow::Result<()> {
        self.a_storage.insert(i, value.clone());
        Ok(())
    }
    async fn get_a(&self, i: I) -> anyhow::Result<Option<A>> {
        Ok(self.a_storage.get(&i).map(|v| v.value().clone()))
    }

    async fn put_b(&self, i: I, j: J, value: &B) -> anyhow::Result<()> {
        self.b_storage.insert((i, j), value.clone());
        Ok(())
    }
    async fn get_b(&self, i: I, j: J) -> anyhow::Result<Option<B>> {
        Ok(self.b_storage.get(&(i, j)).map(|v| v.value().clone()))
    }

    async fn put_c(&self, i: I, k: K, value: &C) -> anyhow::Result<()> {
        self.c_storage.insert((i, k), value.clone());
        Ok(())
    }
    async fn get_c(&self, i: I, k: K) -> anyhow::Result<Option<C>> {
        Ok(self.c_storage.get(&(i, k)).map(|v| v.value().clone()))
    }

    async fn put_d(&self, i: I, j: J, k: K, value: &D) -> anyhow::Result<()> {
        self.d_storage.insert((i, j, k), value.clone());
        Ok(())
    }
    async fn get_d(&self, i: I, j: J, k: K) -> anyhow::Result<Option<D>> {
        Ok(self.d_storage.get(&(i, j, k)).map(|v| v.value().clone()))
    }

    async fn put_e(&self, i: I, k: K, value: &E) -> anyhow::Result<()> {
        self.e_storage.insert((i, k), value.clone());
        Ok(())
    }
    async fn get_e(&self, i: I, k: K) -> anyhow::Result<Option<E>> {
        Ok(self.e_storage.get(&(i, k)).map(|v| v.value().clone()))
    }

    async fn put_f(&self, i: I, value: &F) -> anyhow::Result<()> {
        self.f_storage.insert(i, value.clone());
        Ok(())
    }
    async fn get_f(&self, i: I) -> anyhow::Result<Option<F>> {
        Ok(self.f_storage.get(&i).map(|v| v.value().clone()))
    }
}

// The database storage implementation
// In practice, the details here would be hidden behind something like
// `use_psql_storage!(DataStorage);`
struct DataStorage {
    schema: Option<String>,
    pool: deadpool_postgres::Pool,
}
impl DataStorage {
    async fn new() -> anyhow::Result<Self> {
        // URI and schema would be parsed from configuration files.
        let uri = DATABASE_URI;
        let schema = DATA_SCHEMA.map(|s| s.to_string());
        let pg_config: tokio_postgres::Config = {
            let mut config: tokio_postgres::Config = uri.parse()?;
            config
                .keepalives(true)
                .keepalives_idle(std::time::Duration::from_secs(60))
                .keepalives_interval(std::time::Duration::from_secs(30));
            config
        };
        let manager_config = deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Clean,
        };
        let manager = deadpool_postgres::Manager::from_config(
            pg_config,
            tokio_postgres::NoTls,
            manager_config,
        );
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(16)
            .build()?;
        // Create the schema and tables if they do not exist
        if let Some(schema) = &schema {
            let conn = pool.get().await?;
            conn.batch_execute(
                &format!(
                    "CREATE SCHEMA IF NOT EXISTS {schema}; \
                     CREATE TABLE IF NOT EXISTS {schema}.a (i BIGINT, value JSONB, PRIMARY KEY (i)); \
                     CREATE TABLE IF NOT EXISTS {schema}.b (i BIGINT, j BIGINT, value JSONB, PRIMARY KEY (i, j)); \
                     CREATE TABLE IF NOT EXISTS {schema}.c (i BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, k)); \
                     CREATE TABLE IF NOT EXISTS {schema}.d (i BIGINT, j BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, j, k)); \
                     CREATE TABLE IF NOT EXISTS {schema}.e (i BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, k)); \
                     CREATE TABLE IF NOT EXISTS {schema}.f (i BIGINT, value JSONB, PRIMARY KEY (i)); \
                     CREATE TABLE IF NOT EXISTS {schema}._data_footprint (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
                )
            )
            .await?;
        } else {
            // Create tables without schema
            let conn = pool.get().await?;
            conn.batch_execute(
                "CREATE TABLE IF NOT EXISTS a (i BIGINT, value JSONB, PRIMARY KEY (i)); \
                 CREATE TABLE IF NOT EXISTS b (i BIGINT, j BIGINT, value JSONB, PRIMARY KEY (i, j)); \
                 CREATE TABLE IF NOT EXISTS c (i BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, k)); \
                 CREATE TABLE IF NOT EXISTS d (i BIGINT, j BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, j, k)); \
                 CREATE TABLE IF NOT EXISTS e (i BIGINT, k BIGINT, value JSONB, PRIMARY KEY (i, k)); \
                 CREATE TABLE IF NOT EXISTS f (i BIGINT, value JSONB, PRIMARY KEY (i)); \
                 CREATE TABLE IF NOT EXISTS _data_footprint (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .await?;
        }

        Ok(Self { schema, pool })
    }
}
#[async_trait]
impl OperonStorage for DataStorage {
    async fn clear(&self) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!(
                "TRUNCATE TABLE \
                    {schema_prefix}b, \
                    {schema_prefix}c, \
                    {schema_prefix}d, \
                    {schema_prefix}e, \
                    {schema_prefix}f, \
                    {schema_prefix}_data_footprint"
            ),
            &[],
        )
        .await?;
        Ok(())
    }

    async fn put_a(&self, i: I, value: &A) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}a (i, value) VALUES ($1, $2) ON CONFLICT (i) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_a(&self, i: I) -> anyhow::Result<Option<A>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}a WHERE i = $1"),
                &[&(i as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn put_b(&self, i: I, j: J, value: &B) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}b (i, j, value) VALUES ($1, $2, $3) ON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &(j as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_b(&self, i: I, j: J) -> anyhow::Result<Option<B>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}b WHERE i = $1 AND j = $2"),
                &[&(i as i64), &(j as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn put_c(&self, i: I, k: K, value: &C) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}c (i, k, value) VALUES ($1, $2, $3) ON CONFLICT (i, k) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &(k as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_c(&self, i: I, k: K) -> anyhow::Result<Option<C>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}c WHERE i = $1 AND k = $2"),
                &[&(i as i64), &(k as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn put_d(&self, i: I, j: J, k: K, value: &D) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}d (i, j, k, value) VALUES ($1, $2, $3, $4) ON CONFLICT (i, j, k) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &(j as i64), &(k as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_d(&self, i: I, j: J, k: K) -> anyhow::Result<Option<D>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}d WHERE i = $1 AND j = $2 AND k = $3"),
                &[&(i as i64), &(j as i64), &(k as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn put_e(&self, i: I, k: K, value: &E) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}e (i, k, value) VALUES ($1, $2, $3) ON CONFLICT (i, k) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &(k as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_e(&self, i: I, k: K) -> anyhow::Result<Option<E>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}e WHERE i = $1 AND k = $2"),
                &[&(i as i64), &(k as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn put_f(&self, i: I, value: &F) -> anyhow::Result<()> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}f (i, value) VALUES ($1, $2) ON CONFLICT (i) DO UPDATE SET value = EXCLUDED.value"),
            &[&(i as i64), &serde_json::to_value(value)?],
        )
        .await?;
        Ok(())
    }
    async fn get_f(&self, i: I) -> anyhow::Result<Option<F>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}f WHERE i = $1"),
                &[&(i as i64)],
            )
            .await?;
        row.map(|r| serde_json::from_value(r.get(0)))
            .transpose()
            .map_err(anyhow::Error::from)
    }
    async fn clear_footprint(&self) -> anyhow::Result<()> {
        // Clear the footprint table in the database.
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("TRUNCATE TABLE {schema_prefix}_data_footprint"),
            &[],
        )
        .await?;
        Ok(())
    }
    async fn put_footprint(&self, value: &str) -> anyhow::Result<()> {
        // Put a footprint into the database.
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        conn.execute(
            &format!("INSERT INTO {schema_prefix}_data_footprint (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"),
            &[&"global", &value],
        )
        .await?;
        Ok(())
    }
    async fn get_footprint(&self) -> anyhow::Result<Option<String>> {
        // Get the footprint from the database.
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let row = conn
            .query_opt(
                &format!("SELECT value FROM {schema_prefix}_data_footprint WHERE key = $1"),
                &[&"global"],
            )
            .await?;
        Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
    }

    // Copy-in methods for bulk inserts
    async fn put_all_b(&self, i: I, values: &[B]) -> anyhow::Result<()> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let create_temp_stmt = format!(
            "CREATE TEMP TABLE temp_b (LIKE {schema_prefix}b INCLUDING ALL) ON COMMIT DROP",
        );
        tx.execute(&create_temp_stmt, &[]).await?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        for (j, value) in values.iter().enumerate() {
            writer.serialize((i, j, format!("{}", serde_json::to_value(value)?)))?;
        }
        let copy_stmt = "COPY temp_b (i, j, value) FROM STDIN WITH (FORMAT csv)";
        let sink = tx.copy_in(copy_stmt).await?;
        let mut sink = Box::pin(sink);
        sink.send(bytes::Bytes::from(writer.into_inner()?)).await?;
        sink.close().await?;
        let insert_stmt = format!(
            "INSERT INTO {schema_prefix}b (i, j, value)
            SELECT i, j, value FROM temp_b
            ON CONFLICT (i, j) DO UPDATE SET
                value = EXCLUDED.value",
        );
        tx.execute(&insert_stmt, &[]).await?;
        tx.commit().await?;
        Ok(())
    }
    async fn put_all_c(&self, i: I, values: &[C]) -> anyhow::Result<()> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let create_temp_stmt = format!(
            "CREATE TEMP TABLE temp_c (LIKE {schema_prefix}c INCLUDING ALL) ON COMMIT DROP",
        );
        tx.execute(&create_temp_stmt, &[]).await?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        for (k, value) in values.iter().enumerate() {
            writer.serialize((i, k, format!("{}", serde_json::to_value(value)?)))?;
        }
        let copy_stmt = "COPY temp_c (i, k, value) FROM STDIN WITH (FORMAT csv)";
        let sink = tx.copy_in(copy_stmt).await?;
        let mut sink = Box::pin(sink);
        sink.send(bytes::Bytes::from(writer.into_inner()?)).await?;
        sink.close().await?;
        let insert_stmt = format!(
            "INSERT INTO {schema_prefix}c (i, k, value)
            SELECT i, k, value FROM temp_c
            ON CONFLICT (i, k) DO UPDATE SET
                value = EXCLUDED.value",
        );
        tx.execute(&insert_stmt, &[]).await?;
        tx.commit().await?;
        Ok(())
    }

    // Select with unspecified dimensions for bulk retrieval
    async fn get_all_b_over_j(&self, i: I) -> anyhow::Result<Vec<B>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let rows = conn
            .query(
                &format!("SELECT value FROM {schema_prefix}b WHERE i = $1 ORDER BY j"),
                &[&(i as i64)],
            )
            .await?;
        rows.iter()
            .map(|row| serde_json::from_value(row.get(0)).map_err(anyhow::Error::from))
            .collect()
    }
    async fn get_all_c_over_k(&self, i: I) -> anyhow::Result<Vec<C>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let rows = conn
            .query(
                &format!("SELECT value FROM {schema_prefix}c WHERE i = $1 ORDER BY k"),
                &[&(i as i64)],
            )
            .await?;
        rows.iter()
            .map(|row| serde_json::from_value(row.get(0)).map_err(anyhow::Error::from))
            .collect()
    }
    async fn get_all_d_over_j(&self, i: I, k: K) -> anyhow::Result<Vec<D>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let rows = conn
            .query(
                &format!("SELECT value FROM {schema_prefix}d WHERE i = $1 AND k = $2 ORDER BY j"),
                &[&(i as i64), &(k as i64)],
            )
            .await?;
        rows.iter()
            .map(|row| serde_json::from_value(row.get(0)).map_err(anyhow::Error::from))
            .collect()
    }
    async fn get_all_e_over_k(&self, i: I) -> anyhow::Result<Vec<E>> {
        let conn = self.pool.get().await?;
        let schema_prefix = match &self.schema {
            Some(schema) => format!("{schema}."),
            None => String::new(),
        };
        let rows = conn
            .query(
                &format!("SELECT value FROM {schema_prefix}e WHERE i = $1 ORDER BY k"),
                &[&(i as i64)],
            )
            .await?;
        rows.iter()
            .map(|row| serde_json::from_value(row.get(0)).map_err(anyhow::Error::from))
            .collect()
    }
}

// Example minimal random service implementation
struct ExampleService;
// Implement the OperonService trait
// This must always be implemented by the user.
#[async_trait]
impl OperonService for ExampleService {
    async fn beta(&self, a: &A) -> anyhow::Result<Vec<B>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in beta"));
        // }
        let mut result = Vec::new();
        let mut rng = rand::rng();
        for i in 0..rng.random_range(5..=10) {
            let b = B(a.clone(), a.0.len() + i);
            result.push(b);
        }
        Ok(result)
    }

    async fn gamma(&self, a: &A) -> anyhow::Result<Vec<C>> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in gamma"));
        // }
        let mut result = Vec::new();
        let mut rng = rand::rng();
        let len = a.0.len();
        for _ in 0..rng.random_range(5..=10) {
            let c = C(rng.random_range(1..=len));
            result.push(c);
        }
        Ok(result)
    }

    async fn delta(&self, a: &A, b: &B, c: &C) -> anyhow::Result<D> {
        // // Simulate some processing
        // // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     // panic!("Simulated failure in delta");
        //     return Err(anyhow::anyhow!("Simulated failure in delta"));
        // }
        let d = D {
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
        };
        // // Printing logs should be done using the macros `operon` provides.
        // // For example:
        // let mut rng = rand::rng();
        // if rng.random_bool(0.001) {
        //     operon::info!("Sample delta: {d:?}");
        // }
        log::trace!("Delta computed: {d:?}");
        Ok(d)
    }

    async fn epsilon(&self, b_j: &[B], d_j: &[D]) -> anyhow::Result<E> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in epsilon"));
        // }
        let e = E {
            b: b_j.to_vec(),
            d: d_j.to_vec(),
        };
        Ok(e)
    }

    async fn zeta(&self, c_k: &[C], e_k: &[E]) -> anyhow::Result<F> {
        // Simulate some processing
        // Poison this function to simulate a failure
        // let mut rng = rand::rng();
        // if rng.random_bool(0.0005) {
        //     return Err(anyhow::anyhow!("Simulated failure in zeta"));
        // }
        let mut rng = rand::rng();
        let f = if rng.random_bool(0.5) {
            F::Success {
                c: c_k.to_vec(),
                e: e_k.to_vec(),
            }
        } else {
            F::Failure(
                format!("F (failure with {} C and {} E)", c_k.len(), e_k.len()),
                c_k.to_vec().first().cloned(),
                e_k.to_vec().first().cloned(),
            )
        };
        Ok(f)
    }
}

fn main() -> Result<(), OperonError> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(2 * 1024 * 1024)
        .build()
        .unwrap()
        .block_on(run())
}

async fn run() -> Result<(), OperonError> {
    // Primary upper bound for the dimension I
    let primary_ub = 1000;

    // Create an instance of the DataStorage
    let storage = DataStorage::new().await.map_err(OperonError::Storage)?;
    // Example primary data (these should be preexisting in the database)
    for i in 0..primary_ub {
        let a = A(format!("A ({i})"));
        storage.put_a(i, &a).await.map_err(OperonError::Storage)?;
    }
    // Create an instance of the ExampleService
    let service = ExampleService;

    // Wrap the storage and service in Arc
    let storage = Arc::new(storage);
    let service = Arc::new(service);

    // Create an instance of the Operon
    let operon = {
        let storage = storage.clone();
        let service = service.clone();
        Operon::new(storage, service)
    };

    // Run the Operon
    operon.run(primary_ub).await?;

    Ok(())
}
