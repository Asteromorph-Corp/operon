use std::path::PathBuf;

use proc_macro2::TokenStream;

// Parsed from `types`:

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DimensionTag(pub usize);
#[derive(Debug)]
pub struct Dimension {
    pub tag: DimensionTag,
    pub name: String,
    pub primary: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityTag(pub usize);
#[derive(Debug)]
pub struct Entity {
    pub tag: EntityTag,
    pub name: String,
    pub primary: bool,
    pub dimensions: Vec<DimensionTag>,
    pub definition: Option<(JobTag, Vec<DimensionTag>)>,
    pub from: Option<Vec<(EntityTag, Vec<DimensionTag>)>>,
    pub pool: Option<usize>,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobTag(pub usize);
#[derive(Debug)]
pub struct Job {
    pub tag: JobTag,
    pub name: String,
    pub repeats_on: Vec<DimensionTag>,
    pub from: Vec<(EntityTag, Vec<DimensionTag>)>,
    pub defines: (EntityTag, Vec<DimensionTag>),
    pub spawns: Option<DimensionTag>,
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

#[derive(Debug)]
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
    pub pool_size: Option<usize>,
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

