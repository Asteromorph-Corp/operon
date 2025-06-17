use indexmap::IndexMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::path::PathBuf;
use super::types::*;

static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(get_config_path);

#[derive(Debug, Deserialize)]
struct RawEntity {
    primary: Option<bool>,
    #[serde(rename = "type")]
    r#type: String,
    dimensions: Vec<String>,
    definition: Option<String>,
    from: Option<Vec<String>>,
    pool: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RawTypesConfig {
    entity: IndexMap<String, RawEntity>,
}

#[derive(Debug, Deserialize, Default)]
struct RawLogConfig {
    level: Option<String>,
    buffer_size: Option<usize>,
    dump: Option<bool>,
    dump_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    storage: StorageConfig,
    log: Option<RawLogConfig>,
}

fn read_types_from_str(types_config: &str) -> RawTypesConfig {
    RawTypesConfig::deserialize(toml::de::Deserializer::new(types_config))
        .expect("Failed to parse types file")
}
fn read_config_from_str(config: &str) -> RawConfig {
    RawConfig::deserialize(toml::de::Deserializer::new(config))
        .expect("Failed to parse config file")
}

fn get_config_path() -> PathBuf {
    let mut path = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .expect("CARGO_MANIFEST_DIR not set");
    path.push("config");
    assert!(
        path.exists(),
        "Config directory does not exist at {}",
        path.display()
    );
    path
}

fn parse_entities(raw: RawTypesConfig) -> Entities {
    let mut entities = IndexMap::new();

    // TODO

    entities
}
fn parse_config(raw: RawConfig) -> GlobalConfig {
    let log = raw.log.unwrap_or_default();
    let log_config = LogConfig {
        level: log.level.unwrap_or("info".to_string()),
        buffer_size: log.buffer_size.unwrap_or(1024),
        dump: log.dump.unwrap_or(false),
        dump_path: CONFIG_PATH.join(log.dump_path.unwrap_or("logs".to_string())),
    };

    GlobalConfig {
        storage: raw.storage,
        log: log_config,
    }
}

pub fn get_entities() -> Entities {
    let content =
        std::fs::read_to_string(CONFIG_PATH.join("types.toml")).expect("Couldn't read types.toml");
    let raw = read_types_from_str(&content);
    parse_entities(raw)
}
pub fn get_config() -> GlobalConfig {
    let content = std::fs::read_to_string(CONFIG_PATH.join("config.toml"))
        .expect("Couldn't read config.toml");
    let raw = read_config_from_str(&content);
    parse_config(raw)
}
