use portal::protocol::model::Timestamp;

use crate::AppError;

#[derive(uniffi::Object)]
pub struct NWC {
    inner: nwc::NWC,
}

#[uniffi::export]
impl NWC {
    #[uniffi::constructor]
    pub fn new(uri: String) -> Result<Self, AppError> {
        Ok(Self {
            inner: nwc::NWC::new(
                uri.parse()
                    .map_err(|_| AppError::NWC("Invalid NWC URL".to_string()))?,
            ),
        })
    }

    pub async fn pay_invoice(&self, invoice: String) -> Result<String, AppError> {
        let response = self
            .inner
            .pay_invoice(portal::nostr::nips::nip47::PayInvoiceRequest::new(invoice))
            .await?;

        Ok(response.preimage)
    }

    pub async fn lookup_invoice(&self, invoice: String) -> Result<LookupInvoiceResponse, AppError> {
        let response = self
            .inner
            .lookup_invoice(portal::nostr::nips::nip47::LookupInvoiceRequest {
                invoice: None,
                payment_hash: Some(invoice),
            })
            .await?;

        Ok(response.into())
    }

    pub async fn get_info(&self) -> Result<GetInfoResponse, AppError> {
        let resp = self
            .inner
            .get_info()
            .await
            .map_err(|e| AppError::NWC(e.to_string()))?;
        Ok(GetInfoResponse::from(resp))
    }

    pub async fn get_balance(&self) -> Result<u64, AppError> {
        let balance = self
            .inner
            .get_balance()
            .await
            .map_err(|e| AppError::NWC(e.to_string()))?;
        Ok(balance)
    }
}

#[derive(Debug, uniffi::Record)]
pub struct LookupInvoiceResponse {
    pub transaction_type: Option<TransactionType>,
    pub invoice: Option<String>,
    pub description: Option<String>,
    pub description_hash: Option<String>,
    pub preimage: Option<String>,
    pub payment_hash: String,
    pub amount: u64,
    pub fees_paid: u64,
    pub created_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub settled_at: Option<Timestamp>,
}

impl From<portal::nostr::nips::nip47::LookupInvoiceResponse> for LookupInvoiceResponse {
    fn from(response: portal::nostr::nips::nip47::LookupInvoiceResponse) -> Self {
        Self {
            transaction_type: response.transaction_type.map(|t| t.into()),
            invoice: response.invoice,
            description: response.description,
            description_hash: response.description_hash,
            preimage: response.preimage,
            payment_hash: response.payment_hash,
            amount: response.amount,
            fees_paid: response.fees_paid,
            created_at: Timestamp::new(response.created_at.as_u64()),
            expires_at: response.expires_at.map(|t| Timestamp::new(t.as_u64())),
            settled_at: response.settled_at.map(|t| Timestamp::new(t.as_u64())),
        }
    }
}

#[derive(Debug, uniffi::Enum)]
pub enum TransactionType {
    Incoming,
    Outgoing,
}

impl From<portal::nostr::nips::nip47::TransactionType> for TransactionType {
    fn from(transaction_type: portal::nostr::nips::nip47::TransactionType) -> Self {
        match transaction_type {
            portal::nostr::nips::nip47::TransactionType::Incoming => TransactionType::Incoming,
            portal::nostr::nips::nip47::TransactionType::Outgoing => TransactionType::Outgoing,
        }
    }
}

/// Get Info Response
#[derive(Debug, uniffi::Record)]
pub struct GetInfoResponse {
    pub alias: Option<String>,
    pub color: Option<String>,
    pub pubkey: Option<String>,
    pub network: Option<String>,
    pub block_height: Option<u32>,
    pub block_hash: Option<String>,
    pub methods: Vec<String>,
    pub notifications: Vec<String>,
}

impl From<portal::nostr::nips::nip47::GetInfoResponse> for GetInfoResponse {
    fn from(response: portal::nostr::nips::nip47::GetInfoResponse) -> Self {
        Self {
            alias: response.alias,
            color: response.color,
            pubkey: response.pubkey.map(|pk| pk.to_string()), // Convert PublicKey to String
            network: response.network,
            block_height: response.block_height,
            block_hash: response.block_hash,
            methods: response.methods,
            notifications: response.notifications,
        }
    }
}
