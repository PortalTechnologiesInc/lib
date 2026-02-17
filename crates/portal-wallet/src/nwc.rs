use std::sync::Arc;

use axum::async_trait;
use nwc::NWC;

use crate::{PortalWallet, Result};

/// NWC Wallet implementation
pub struct NwcWallet {
    nwc: Arc<NWC>,
}

impl NwcWallet {
    pub fn new(nwc_url: String) -> Result<Self> {
        Ok(Self {
            nwc: Arc::new(NWC::new(nwc_url.parse()?)),
        })
    }
}

#[async_trait]
impl PortalWallet for NwcWallet {
    async fn make_invoice(&self, sats: u64, description: Option<String>) -> Result<String> {
        let payment_response = self
            .nwc
            .make_invoice(portal::nostr::nips::nip47::MakeInvoiceRequest {
                amount: sats,
                description: description,
                description_hash: None,
                expiry: None,
            })
            .await?;

        Ok(payment_response.invoice)
    }

    async fn pay_invoice(&self, invoice: String) -> Result<String> {
        let response = self
            .nwc
            .pay_invoice(portal::nostr::nips::nip47::PayInvoiceRequest::new(invoice))
            .await?;

        Ok(response.preimage)
    }

    async fn is_invoice_paid(&self, invoice: String) -> Result<(bool, Option<String>)> {
        let invoice = self
            .nwc
            .lookup_invoice(portal::nostr::nips::nip47::LookupInvoiceRequest {
                invoice: Some(invoice),
                payment_hash: None,
            })
            .await?;

        Ok((invoice.settled_at.is_some(), invoice.preimage))
    }
}
