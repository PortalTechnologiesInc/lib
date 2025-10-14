use axum::async_trait;

#[async_trait]
pub trait PortalWallet: Send + Sync {
    async fn make_invoice(&self, sats: u64, description: Option<String>) -> anyhow::Result<String>;
    async fn is_invoice_paid(&self, invoice: String) -> anyhow::Result<(bool, Option<String>)>;
}

pub mod breez;
pub mod nwc;
