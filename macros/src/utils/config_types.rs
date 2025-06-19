use std::path::PathBuf;

// Parsed from `types`:

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionTag(pub usize);
/// An Operon dimension.
#[derive(Debug)]
pub struct Dimension {
    /// Unique identifier for the dimension.
    pub tag: DimensionTag,
    /// Name of the dimension.
    pub name: String,
    /// Whether this dimension is the primary dimension.
    pub primary: bool,
    /// Upstream dimensions that this dimension depends on.
    pub depends_on: Vec<DimensionTag>,
}
#[allow(dead_code)]
impl Dimension {
    pub fn get_by_tag(dimensions: &Dimensions, tag: DimensionTag) -> Option<&Dimension> {
        let dimension = dimensions.get(tag.0);
        if dimension.is_some() {
            assert_eq!(dimension.unwrap().tag, tag, "Dimension tag mismatch");
        }
        dimension
    }
    pub fn get_by_name<'a> (dimensions: &'a Dimensions, name: &str) -> Option<&'a Dimension> {
        dimensions.iter().find(|d| d.name == name)
    }
}
pub type Dimensions = Vec<Dimension>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityTag(pub usize);
/// An Operon entity, which can be a struct or an enum.
#[derive(Debug)]
pub struct Entity {
    /// Unique identifier for the entity.
    pub tag: EntityTag,
    /// Name of the entity.
    pub name: String,
    /// Whether this entity is the primary entity.
    pub primary: bool,
    /// Dimensions this entity repeat on.
    pub dims: Vec<DimensionTag>,
    /// The job that defines this entity.
    /// This is `None` if and only if this entity is primary.
    pub def: Option<JobTag>,
    /// The entities this entity is derived from.
    /// This is `None` if and only if this entity is primary.
    pub from: Option<Vec<EntityTag>>,
    /// The Rust definition of this entity.
    pub body: String,
}
#[allow(dead_code)]
impl Entity {
    pub fn get_by_tag(entities: &Entities, tag: EntityTag) -> Option<&Entity> {
        let entity = entities.get(tag.0);
        if entity.is_some() {
            assert_eq!(entity.unwrap().tag, tag, "Entity tag mismatch");
        }
        entity
    }
    pub fn get_by_name<'a> (entities: &'a Entities, name: &str) -> Option<&'a Entity> {
        entities.iter().find(|e| e.name == name)
    }
}
pub type Entities = Vec<Entity>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobTag(pub usize);
/// An Operon job.
#[derive(Debug)]
pub struct Job {
    /// Unique identifier for the job.
    pub tag: JobTag,
    /// Name of the job.
    pub name: String,
    /// The dimensions this job repeats on.
    pub repeats_on: Vec<DimensionTag>,
    /// The *input* entities of this job.
    /// An entity paired with `n` dimensions means that
    /// the job will accept an `n`-dimensional slice of that entity.
    pub from: Vec<(EntityTag, Vec<DimensionTag>)>,
    /// Which entity this job *outputs* (or *defines*).
    /// Like `from`, this is a pair of the entity tag and the dimensions it repeats on.
    /// In this case, `n` dimensions means that
    /// the job will return an `n`-dimensional `Vec` of that entity.
    /// 
    /// Since non-primary entities are always uniquely linked to a job,
    /// `tag.0 + 1 == defines.0.0` will always hold.
    pub defines: (EntityTag, Vec<DimensionTag>),
    /// The dimension that this job spawns.
    /// This being an `Option` means that jobs can only spawn at most one dimension at a time.
    pub spawns: Option<DimensionTag>,
    /// How many instances of this job can run in parallel.
    pub pool: usize,
}
impl Job {
    pub fn get_by_tag(jobs: &Jobs, tag: JobTag) -> Option<&Job> {
        let job = jobs.get(tag.0);
        if job.is_some() {
            assert_eq!(job.unwrap().tag, tag, "Job tag mismatch");
        }
        job
    }
    pub fn get_by_name<'a> (jobs: &'a Jobs, name: &str) -> Option<&'a Job> {
        jobs.iter().find(|j| j.name == name)
    }
}
pub type Jobs = Vec<Job>;

#[derive(Debug, Default)]
pub struct TypesConfig {
    pub entities: Entities,
    pub dimensions: Dimensions,
    pub jobs: Jobs,
}

// Parsed from `config`:

#[derive(Debug, Default)]
pub struct Connection {
    pub uri: String,
    pub schema: Option<String>,
    pub pool_size: usize,
}

#[derive(Debug, Default)]
pub struct StorageConfig {
    pub metadata: Connection,
    pub data: Connection,
}

#[derive(Debug, Default)]
pub struct LogConfig {
    pub level: String,
    pub buffer_size: usize,
    pub dump: bool,
    pub dump_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct CommonConfig {
    pub storage: StorageConfig,
    pub log: LogConfig,
}

/// All configuration parsed from the macro input.
/// 
/// This struct is used to hold the common configuration and the types configuration.
/// 
/// Provided configurations are:
/// 
/// * `common.storage.metadata.uri` - URI for the metadata storage.
/// * `common.storage.metadata.schema` - Optional schema for the metadata storage.
/// * `common.storage.metadata.pool_size` - Connection pool size for the metadata storage.
/// * `common.storage.data.uri` - URI for the data storage.
/// * `common.storage.data.schema` - Optional schema for the data storage.
/// * `common.storage.data.pool_size` - Connection pool size for the data storage.
/// * `common.log.level` - Log level (e.g., "trace", "debug", "info", "warn", "error").
/// * `common.log.buffer_size` - Size of the log buffer in the UI.
/// * `common.log.dump` - Whether to dump logs to a file.
/// * `common.log.dump_path` - Path to the log dump directory.
/// * `types.entities` - List of entities defined in the macro input.
/// * `types.dimensions` - List of dimensions defined in the macro input.
/// * `types.jobs` - List of jobs defined in the macro input.
/// * `use_psql_storage` - Optional string indicating the name of the `OperonStorage` default implementation.
#[derive(Debug, Default)]
pub struct AllConfig {
    pub common: CommonConfig,
    pub types: TypesConfig,
    pub use_psql_storage: Option<String>,
}
