use config::{Config, Environment, File};
use serde::{Deserialize, Deserializer};
use std::sync::Arc;
use tracing::info;
use wallet::{BreezSparkWallet, NwcWallet, PortalWallet};

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub info: InfoSettings,
    pub nostr: NostrSettings,
    pub auth: AuthSettings,
    pub wallet: WalletSettings,
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
    pub storage_dir: String,
    pub mnemonic: String,
}

impl Default for BreezSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            storage_dir: String::from("~/.breez"),
            mnemonic: String::new(),
        }
    }
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let config_file = dirs::home_dir()
            .ok_or(anyhow::anyhow!("Home directory not found"))?
            .join(".portal-rest")
            .join("config.toml");

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
                let breez = BreezSparkWallet::new(
                    settings.api_key.clone(),
                    settings.storage_dir.clone(),
                    settings.mnemonic.clone(),
                )
                .await?;

                info!("Breez Wallet created");
                anyhow::Ok(Some(Arc::new(breez)))
            }
        }
    }
}
