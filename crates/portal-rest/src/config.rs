use config::{Config, Environment, File};
use portal_wallet::{BreezSparkWallet, NwcWallet, PortalWallet};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub info: InfoSettings,
    pub nostr: NostrSettings,
    pub auth: AuthSettings,
    pub wallet: WalletSettings,
    #[serde(default)]
    pub webhook: WebhookSettings,
    #[serde(default)]
    pub database: DatabaseSettings,
    #[serde(default)]
    pub profile: ProfileSettings,
    #[serde(default)]
    pub logging: LoggingSettings,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoggingSettings {
    /// `tracing` / `RUST_LOG` filter (e.g. `info`, `debug`, `portal_rest=debug`).
    /// Used only when `RUST_LOG` is not set in the process environment.
    #[serde(default = "default_log_filter")]
    pub filter: String,
}

fn default_log_filter() -> String {
    "info".to_string()
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            filter: default_log_filter(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ProfileSettings {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub picture: Option<String>,
    pub nip05: Option<String>,
}

impl ProfileSettings {
    /// Returns true if any field is set.
    pub fn is_set(&self) -> bool {
        self.name.is_some()
            || self.display_name.is_some()
            || self.picture.is_some()
            || self.nip05.is_some()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InfoSettings {
    pub listen_port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NostrSettings {
    pub private_key: String,
    pub relays: Vec<String>,
    pub subkey_proof: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AuthSettings {
    pub auth_token: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WalletSettings {
    pub ln_backend: LnBackend,
    pub nwc: Option<NwcSettings>,
    pub breez: Option<BreezSettings>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct WebhookSettings {
    pub url: Option<String>,
    pub secret: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseSettings {
    pub path: String,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            path: "portal-rest.db".to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LnBackend {
    None,
    Nwc,
    Breez,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NwcSettings {
    pub url: String,
}

impl Default for NwcSettings {
    fn default() -> Self {
        Self { url: String::new() }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BreezSettings {
    pub api_key: String,
    pub mnemonic: String,
}

impl Default for BreezSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            mnemonic: String::new(),
        }
    }
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let config_file = crate::constants::portal_rest_dir()?.join("config.toml");

        // Create default config if it doesn't exist
        if !config_file.exists() {
            std::fs::create_dir_all(
                config_file
                    .parent()
                    .ok_or(anyhow::anyhow!("Config file parent directory not found"))?,
            )?;
            std::fs::write(&config_file, include_str!("../example.config.toml"))?;
        }

        let settings: Settings = Config::builder()
            // Load from TOML file
            .add_source(File::from(config_file))
            // Override with env vars prefixed with PORTAL_
            // e.g. PORTAL_NOSTR__PRIVATE_KEY -> nostr.private_key
            //      PORTAL_WALLET__LN_BACKEND -> wallet.ln_backend
            .add_source(
                Environment::with_prefix("PORTAL")
                    .prefix_separator("__") // double underscore after PORTAL
                    .separator("__") // double underscore for nested keys
                    .list_separator(",") // comma separator for list values
                    .with_list_parse_key("nostr.relays")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()?;

        Ok(settings)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        match self.wallet.ln_backend {
            LnBackend::None => anyhow::Ok(()),
            LnBackend::Nwc => {
                if self.wallet.nwc.is_none() {
                    return Err(anyhow::anyhow!("NWC Wallet is not set"));
                }
                anyhow::Ok(())
            }
            LnBackend::Breez => {
                if self.wallet.breez.is_none() {
                    return Err(anyhow::anyhow!("Breez Wallet is not set"));
                }
                anyhow::Ok(())
            }
        }
    }

    pub async fn build_wallet(&self) -> anyhow::Result<Option<Arc<dyn PortalWallet>>> {
        match self.wallet.ln_backend {
            LnBackend::None => {
                info!("No wallet configured");
                anyhow::Ok(None)
            }
            LnBackend::Nwc => {
                let settings = self
                    .wallet
                    .nwc
                    .as_ref()
                    .ok_or(anyhow::anyhow!("NWC Wallet is not set"))?;
                let nwc = NwcWallet::new(settings.url.clone())?;

                info!("NWC Wallet created");
                anyhow::Ok(Some(Arc::new(nwc)))
            }
            LnBackend::Breez => {
                let settings = self
                    .wallet
                    .breez
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Breez Wallet is not set"))?;
                let storage_dir = crate::constants::portal_rest_dir()?
                    .join("breez")
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid portal-rest path"))?
                    .to_string();
                std::fs::create_dir_all(&storage_dir)?;
                let breez = BreezSparkWallet::new(
                    settings.api_key.clone(),
                    storage_dir,
                    settings.mnemonic.clone(),
                )
                .await?;

                info!("Breez Wallet created");
                anyhow::Ok(Some(Arc::new(breez)))
            }
        }
    }
}
