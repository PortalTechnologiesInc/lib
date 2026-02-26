//! Config loaded from ~/.portal-app-demo/config.toml. Breez data under ~/.portal-app-demo/breez.

use config::{Config, Environment, File};
use portal_wallet::{BreezSparkWallet, NwcWallet, PortalWallet};
use serde::Deserialize;
use std::sync::Arc;

use crate::constants;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub info: InfoSettings,
    pub identity: IdentitySettings,
    pub nostr: NostrSettings,
    pub wallet: WalletSettings,
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
pub struct IdentitySettings {
    #[serde(default)]
    pub mnemonic: Option<String>,
    #[serde(default)]
    pub nsec: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NostrSettings {
    #[serde(default = "default_relays")]
    pub relays: Vec<String>,
}

fn default_relays() -> Vec<String> {
    vec![
        "wss://relay.nostr.net".into(),
        "wss://relay.getportal.cc".into(),
    ]
}

#[derive(Deserialize, Debug, Clone)]
pub struct WalletSettings {
    #[serde(default)]
    pub ln_backend: LnBackend,
    #[serde(default)]
    pub nwc: Option<NwcSettings>,
    #[serde(default)]
    pub breez: Option<BreezSettings>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum LnBackend {
    #[default]
    None,
    Nwc,
    Breez,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NwcSettings {
    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BreezSettings {
    pub api_key: String,
    pub mnemonic: String,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = constants::portal_app_demo_dir()?;
        let config_file = config_dir.join("config.toml");

        if !config_file.exists() {
            std::fs::create_dir_all(&config_dir)?;
            std::fs::write(&config_file, include_str!("../example.config.toml"))?;
            anyhow::bail!(
                "Created config at {}. Edit it (identity + optional wallet) and restart.",
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
                    .with_list_parse_key("nostr.relays")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()?;

        Ok(settings)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let has_identity = self.identity.mnemonic.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
            || self.identity.nsec.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
        if !has_identity {
            anyhow::bail!("Set identity.mnemonic or identity.nsec in config");
        }
        match self.wallet.ln_backend {
            LnBackend::None => {}
            LnBackend::Nwc => {
                if self.wallet.nwc.as_ref().map(|n| n.url.is_empty()).unwrap_or(true) {
                    anyhow::bail!("wallet.ln_backend is nwc but wallet.nwc.url is not set");
                }
            }
            LnBackend::Breez => {
                if self.wallet.breez.is_none() {
                    anyhow::bail!("wallet.ln_backend is breez but [wallet.breez] is not set");
                }
            }
        }
        Ok(())
    }

    /// Build payment wallet from config. Breez storage is always ~/.portal-app-demo/breez.
    pub async fn build_payment_wallet(&self) -> anyhow::Result<Option<Arc<dyn PortalWallet>>> {
        match self.wallet.ln_backend {
            LnBackend::None => Ok(None),
            LnBackend::Nwc => {
                let nwc = self
                    .wallet
                    .nwc
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("wallet.nwc not set"))?;
                let w = NwcWallet::new(nwc.url.clone())?;
                log::info!("Payment wallet: NWC");
                Ok(Some(Arc::new(w)))
            }
            LnBackend::Breez => {
                let b = self
                    .wallet
                    .breez
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("wallet.breez not set"))?;
                let storage_dir = constants::breez_storage_dir()?
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid Breez storage path"))?
                    .to_string();
                std::fs::create_dir_all(&storage_dir)?;
                let w = BreezSparkWallet::new(
                    b.api_key.clone(),
                    storage_dir,
                    b.mnemonic.clone(),
                )
                .await?;
                log::info!("Payment wallet: Breez (storage: ~/.portal-app-demo/breez)");
                Ok(Some(Arc::new(w)))
            }
        }
    }
}
