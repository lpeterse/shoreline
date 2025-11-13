use serde::{Deserialize, Serialize};
use shoreline_dht::Id;
use std::path::PathBuf;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub dht: DhtConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DhtConfig {
    pub node_id: Id,
    pub bind_port: u16,
}

impl Config {
    pub async fn load() -> Result<Self, Error> {
        let dir = Self::dir().await?;
        let path = dir.join("config.toml");

        if !path.exists() {
            let default = Config::default();
            let content = toml::to_string_pretty(&default)?;
            tokio::fs::write(path, content).await?;
            Ok(default)
        } else {
            let content = tokio::fs::read_to_string(path).await?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        }
    }

    pub async fn dir() -> Result<PathBuf, Error> {
        let dir1 = std::env::var("SHORELINE_CONFIG_DIR").ok().map(|x| PathBuf::from(x));
        let dir2 = std::env::current_dir().ok().map(|x| x.join(".shoreline"));
        let dir3 = std::env::home_dir().map(|x| x.join(".shoreline"));

        let dir = match (dir1, dir2, dir3) {
            (Some(p), _, _) => p,
            (_, Some(p), _) if p.exists() => p,
            (_, _, Some(p)) => p,
            _ => {
                return Err(
                    "Could not determine config path; set $SHORELINE_CONFIG_DIR or run from home directory".into()
                );
            }
        };

        tokio::fs::create_dir_all(&dir).await?;

        Ok(dir)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { dht: DhtConfig { node_id: Id::random(), bind_port: 6881 } }
    }
}
