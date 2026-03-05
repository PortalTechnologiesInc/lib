//! Config loaded from ~/.portal-app-demo/config.toml.

use config::{Config, Environment, File};
use serde::Deserialize;

use crate::constants;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub info: InfoSettings,
    pub nostr: NostrSettings,
    #[serde(default)]
    pub session: SessionSettings,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InfoSettings {
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
}

fn default_listen_port() -> u16 {
    3030
}

#[derive(Deserialize, Debug, Clone)]
pub struct NostrSettings {
    #[serde(default = "default_relays")]
    pub default_relays: Vec<String>,
}

fn default_relays() -> Vec<String> {
    vec![
        "wss://relay.damus.io".into(),
        "wss://relay.nostr.band".into(),
    ]
}

#[derive(Deserialize, Debug, Clone)]
pub struct SessionSettings {
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,
    #[serde(default = "default_ttl_secs")]
    pub ttl_secs: u64,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            max_sessions: default_max_sessions(),
            ttl_secs: default_ttl_secs(),
        }
    }
}

fn default_max_sessions() -> usize {
    50
}

fn default_ttl_secs() -> u64 {
    7200
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = constants::portal_app_demo_dir()?;
        let config_file = config_dir.join("config.toml");

        if !config_file.exists() {
            std::fs::create_dir_all(&config_dir)?;
            std::fs::write(&config_file, include_str!("../example.config.toml"))?;
            anyhow::bail!(
                "Created config at {}. Edit it and restart.",
                config_file.display()
            );
        }

        let settings: Settings = Config::builder()
            .add_source(File::from(config_file))
            .add_source(
                Environment::with_prefix("PORTAL_APP_DEMO")
                    .prefix_separator("__")
                    .separator("__")
                    .list_separator(",")
                    .with_list_parse_key("nostr.default_relays")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()?;

        Ok(settings)
    }
}
