mod breez;
mod nwc;

use axum::async_trait;
use portal::protocol::model::payment::Millisats;

pub use breez::BreezSparkWallet;
pub use nwc::NwcWallet;

/// Portal Wallet trait
#[async_trait]
pub trait PortalWallet: Send + Sync {
    /// Create an invoice for the given amount (in millisatoshis).
    async fn make_invoice(&self, amount_msat: u64, description: Option<String>) -> Result<String>;
    async fn is_invoice_paid(&self, invoice: String) -> Result<(bool, Option<String>)>;
    /// Get balance (msat)
    async fn get_balance(&self) -> Result<u64>;
    /// Pay invoice, returns (preimage, fees_paid_msat)
    async fn pay_invoice(&self, invoice: String) -> Result<(String, u64)>;

    /// Typed helper (non-breaking): same as `make_invoice` but explicit unit.
    async fn make_invoice_msat(
        &self,
        amount: Millisats,
        description: Option<String>,
    ) -> Result<String> {
        self.make_invoice(amount.as_u64(), description).await
    }

    /// Typed helper (non-breaking): balance in explicit msat unit.
    async fn get_balance_msat(&self) -> Result<Millisats> {
        self.get_balance().await.map(Millisats::new)
    }

    /// Typed helper (non-breaking): returns typed fees in explicit msat unit.
    async fn pay_invoice_msat(&self, invoice: String) -> Result<(String, Millisats)> {
        self.pay_invoice(invoice)
            .await
            .map(|(preimage, fees_msat)| (preimage, Millisats::new(fees_msat)))
    }
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
