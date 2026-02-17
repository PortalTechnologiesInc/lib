mod breez;
mod nwc;

use axum::async_trait;

pub use breez::BreezSparkWallet;
pub use nwc::NwcWallet;

/// Portal Wallet trait
#[async_trait]
pub trait PortalWallet: Send + Sync {
    async fn make_invoice(&self, sats: u64, description: Option<String>) -> Result<String>;
    async fn is_invoice_paid(&self, invoice: String) -> Result<(bool, Option<String>)>;
    /// Get balance (msat)
    async fn get_balance(&self) -> Result<u64>;
    async fn pay_invoice(&self, invoice: String) -> Result<String>;
}

/// Result type for Portal Wallet operations
pub type Result<T> = std::result::Result<T, PortalWalletError>;

/// Portal Wallet error enum
#[derive(Debug, thiserror::Error)]
pub enum PortalWalletError {
    #[error("NIP47 error: {0}")]
    NIP47Error(portal::nostr::nips::nip47::Error),
    #[error("NWC error: {0}")]
    NWCError(::nwc::Error),
    #[error("Breez error: {0}")]
    BreezError(breez_sdk_spark::SdkError),
    #[error("Fee too high: {0}")]
    FeeTooHigh(String),
}

impl From<portal::nostr::nips::nip47::Error> for PortalWalletError {
    fn from(error: portal::nostr::nips::nip47::Error) -> Self {
        PortalWalletError::NIP47Error(error)
    }
}

impl From<::nwc::Error> for PortalWalletError {
    fn from(error: ::nwc::Error) -> Self {
        PortalWalletError::NWCError(error)
    }
}

impl From<breez_sdk_spark::SdkError> for PortalWalletError {
    fn from(error: breez_sdk_spark::SdkError) -> Self {
        PortalWalletError::BreezError(error)
    }
}
