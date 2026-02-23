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
    pub verification: Option<VerificationSettings>,
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

#[derive(Deserialize, Debug, Clone)]
pub struct VerificationSettings {
    pub service_url: String,
    pub api_key: String,
    pub mint_url: String,
    #[serde(default = "default_mint_unit")]
    pub mint_unit: String,
    #[serde(default = "default_mint_amount")]
    pub default_mint_amount: u64,
}

fn default_mint_unit() -> String {
    "multi".to_string()
}

fn default_mint_amount() -> u64 {
    100
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
