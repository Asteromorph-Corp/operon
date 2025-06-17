use std::{path::PathBuf, sync::Arc};
use serde::Deserialize;
use indexmap::IndexMap;

#[derive(Debug)]
pub struct Dimension {
    pub name: String,
    pub primary: bool,
}
#[derive(Debug)]
pub struct Entity {
    pub name: String,
    pub primary: bool,
    pub typedef: String,
    pub dimensions: Vec<Dimension>,
    pub definition: Option<(String, Vec<Dimension>)>,
    pub from: Option<Vec<(Arc<Entity>, Vec<Dimension>)>>,
    pub pool: Option<usize>,
}
pub type Entities = IndexMap<String, Entity>;

#[derive(Debug, Deserialize)]
pub struct Connection {
    pub uri: String,
    pub schema: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub metadata: Connection,
    pub data: Connection,
}

#[derive(Debug)]
pub struct LogConfig {
    pub level: String,
    pub buffer_size: usize,
    pub dump: bool,
    pub dump_path: PathBuf,
}

#[derive(Debug)]
pub struct GlobalConfig {
    pub storage: StorageConfig,
    pub log: LogConfig,
}

