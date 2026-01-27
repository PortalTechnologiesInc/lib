use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use cdk::cdk_database::WalletDatabase;
use cdk::wallet::SendKind;
use cdk::{
    Amount,
    nuts::CurrencyUnit,
    wallet::{SendOptions, Wallet, WalletBuilder},
};
use cdk_common::Token;
use cdk_common::amount::SplitTarget;
use cdk_common::mint_url::MintUrl;
use thiserror::Error;

#[derive(Debug, Error, uniffi::Error)]
pub enum CashuWalletError {
    #[error("Wallet error: {0}")]
    WalletError(String),
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<cdk::error::Error> for CashuWalletError {
    fn from(error: cdk::error::Error) -> Self {
        CashuWalletError::WalletError(error.to_string())
    }
}

#[derive(uniffi::Enum)]
pub enum UnitKind {
    Event {
        date: Option<String>,
        location: Option<String>,
    },
    Other,
}

impl From<cdk::types::UnitKind> for UnitKind {
    fn from(kind: cdk::types::UnitKind) -> Self {
        match kind {
            cdk::types::UnitKind::Event { date, location } => UnitKind::Event { date, location },
            cdk::types::UnitKind::Other => UnitKind::Other,
        }
    }
}

#[derive(uniffi::Record)]
pub struct UnitInfo {
    pub front_card_background: Option<String>,
    pub back_card_background: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub kind: UnitKind,
    pub show_individually: bool,
}

impl From<cdk::types::UnitMetadata> for UnitInfo {
    fn from(info: cdk::types::UnitMetadata) -> Self {
        UnitInfo {
            front_card_background: info.front_card_background,
            back_card_background: info.back_card_background,
            title: info.title,
            description: info.description,
            kind: info.kind.into(),
            show_individually: info.show_individually,
        }
    }
}

#[derive(uniffi::Record)]
pub struct TokenInfo {
    pub mint_url: String,
    pub unit: String,
    pub amount: u64,
}

#[derive(uniffi::Record)]
pub struct ProofInfo {
    pub amount: u64,
    pub keyset_id: String,
    pub c: String,
}

#[uniffi::export]
pub async fn parse_cashu_token(token_str: &str) -> Result<TokenInfo, CashuWalletError> {
    let token =
        Token::from_str(token_str).map_err(|e| CashuWalletError::WalletError(e.to_string()))?;

    Ok(TokenInfo {
        mint_url: token
            .mint_url()
            .map_err(|e| CashuWalletError::WalletError(e.to_string()))?
            .to_string(),
        unit: token
            .unit()
            .ok_or_else(|| CashuWalletError::WalletError("Unit not found".to_string()))?
            .to_string(),
        amount: token
            .value()
            .map_err(|e| CashuWalletError::WalletError(e.to_string()))?
            .as_ref()
            .clone(),
    })
}

#[derive(uniffi::Object)]
pub struct CashuWallet {
    inner: Wallet,
    unit: CurrencyUnit,
}

#[uniffi::export]
impl CashuWallet {
    #[uniffi::constructor]
    pub async fn new(
        mint_url: &str,
        unit: &str,
        seed: Vec<u8>,
        localstore: Arc<dyn CashuLocalStore>,
    ) -> Result<Arc<Self>, CashuWalletError> {
        let currency_unit = CurrencyUnit::from_str(unit)
            .map_err(|e| CashuWalletError::WalletError(format!("Invalid currency unit: {e}")))?;

        let mint_url = MintUrl::from_str(mint_url)
            .map_err(|e| CashuWalletError::WalletError(e.to_string()))?;

        // Wrap the app localstore in the adapter
        let localstore_adapter = Arc::new(AppCashuLocalStore { inner: localstore })
            as Arc<dyn WalletDatabase<Err = cdk::cdk_database::Error> + Send + Sync>;

        let wallet = WalletBuilder::new()
            .mint_url(mint_url)
            .unit(currency_unit.clone())
            .localstore(localstore_adapter)
            .seed(&seed)
            .is_pre_derived(true)
            .target_proof_count(3)
            .build()
            .map_err(|e| CashuWalletError::WalletError(e.to_string()))?;

        Ok(Arc::new(Self {
            inner: wallet,
            unit: currency_unit,
        }))
    }

    /// Get the total balance of the wallet
    pub async fn get_balance(self: Arc<Self>) -> Result<u64, CashuWalletError> {
        async_utility::task::spawn(async move {
            let balance = self.inner.total_balance().await?;
            Ok(balance.as_ref().clone())
        })
        .join()
        .await
        .expect("No async task issues")
    }

    /// Receive a cashu token
    pub async fn receive_token(
        self: Arc<Self>,
        token_str: String,
    ) -> Result<u64, CashuWalletError> {
        async_utility::task::spawn(async move {
            let received_amount = self
                .inner
                .receive(&token_str, cdk::wallet::ReceiveOptions::default())
                .await?;
            Ok(received_amount.as_ref().clone())
        })
        .join()
        .await
        .expect("No async task issues")
    }

    /// Send tokens using a prepared send (simplified - directly send)
    pub async fn send_amount(self: Arc<Self>, amount: u64) -> Result<String, CashuWalletError> {
        async_utility::task::spawn(async move {
            let amount = Amount::from(amount);
            let opts = SendOptions {
                send_kind: SendKind::OfflineExact,
                amount_split_target: SplitTarget::Value(Amount::from(1)),
                ..Default::default()
            };
            let prepared_send = self.inner.prepare_send(amount, opts).await?;
            let token = self.inner.send(prepared_send, None).await?;
            Ok(token.to_string())
        })
        .join()
        .await
        .expect("No async task issues")
    }

    /// Get info about the token
    pub async fn get_unit_info(self: Arc<Self>) -> Result<Option<UnitInfo>, CashuWalletError> {
        async_utility::task::spawn(async move {
            let unit_info = self.inner.get_unit_metadata().await?.map(UnitInfo::from);
            Ok(unit_info)
        })
        .join()
        .await
        .expect("No async task issues")
    }

    /// Get mint URL
    pub fn get_mint_url(&self) -> String {
        self.inner.mint_url.to_string()
    }

    /// Get currency unit
    pub fn get_currency_unit(&self) -> String {
        match &self.unit {
            CurrencyUnit::Sat => "sat".to_string(),
            CurrencyUnit::Msat => "msat".to_string(),
            CurrencyUnit::Usd => "usd".to_string(),
            CurrencyUnit::Eur => "eur".to_string(),
            CurrencyUnit::Auth => "auth".to_string(),
            CurrencyUnit::Custom(s) => s.clone(),
            _ => self.unit.to_string(),
        }
    }

    /// Restore proofs by checking with the mint
    /// This method attempts to restore any unspent proofs that may have been lost
    /// by checking with the mint and reconstructing proofs from the wallet's seed
    pub async fn restore_proofs(self: Arc<Self>) -> Result<u64, CashuWalletError> {
        async_utility::task::spawn(async move {
            let restored_amount = self.inner.restore().await?;
            Ok(restored_amount.as_ref().clone())
        })
        .join()
        .await
        .expect("No async task issues")
    }

    pub fn mint_url(&self) -> String {
        self.inner.mint_url.to_string()
    }

    pub fn unit(&self) -> String {
        self.unit.to_string()
    }
}

impl std::fmt::Debug for CashuWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CashuWallet")
            .field("mint_url", &self.inner.mint_url)
            .field("unit", &self.unit)
            .finish()
    }
}

#[uniffi::export(with_foreign)]
#[async_trait]
pub trait CashuLocalStore: Send + Sync {
    async fn get_proofs(
        &self,
        mint_url: Option<String>,
        unit: Option<String>,
        state: Option<String>,
        spending_conditions: Option<String>,
    ) -> Result<Vec<String>, CashuWalletError>;
    async fn update_proofs(
        &self,
        added: Vec<String>,
        removed_ys: Vec<String>,
    ) -> Result<(), CashuWalletError>;
    async fn update_proofs_state(
        &self,
        ys: Vec<String>,
        state: String,
    ) -> Result<(), CashuWalletError>;
    async fn add_transaction(&self, transaction: String) -> Result<(), CashuWalletError>;
    async fn get_transaction(
        &self,
        transaction_id: String,
    ) -> Result<Option<String>, CashuWalletError>;
    async fn list_transactions(
        &self,
        mint_url: Option<String>,
        direction: Option<String>,
        unit: Option<String>,
    ) -> Result<Vec<String>, CashuWalletError>;
    async fn remove_transaction(&self, transaction_id: String) -> Result<(), CashuWalletError>;
    async fn add_mint(
        &self,
        mint_url: String,
        mint_info: Option<String>,
    ) -> Result<(), CashuWalletError>;
    async fn remove_mint(&self, mint_url: String) -> Result<(), CashuWalletError>;
    async fn get_mint(&self, mint_url: String) -> Result<Option<String>, CashuWalletError>;
    async fn get_mints(&self) -> Result<Vec<String>, CashuWalletError>; // Returns list of mint URLs
    async fn update_mint_url(
        &self,
        old_mint_url: String,
        new_mint_url: String,
    ) -> Result<(), CashuWalletError>;
    async fn add_mint_keysets(
        &self,
        mint_url: String,
        keysets: Vec<String>,
    ) -> Result<(), CashuWalletError>;
    async fn get_mint_keysets(
        &self,
        mint_url: String,
    ) -> Result<Option<Vec<String>>, CashuWalletError>;
    async fn get_keyset_by_id(&self, keyset_id: String)
    -> Result<Option<String>, CashuWalletError>;
    async fn add_keys(&self, keyset: String) -> Result<(), CashuWalletError>;
    async fn get_keys(&self, id: String) -> Result<Option<String>, CashuWalletError>;
    async fn remove_keys(&self, id: String) -> Result<(), CashuWalletError>;
    async fn increment_keyset_counter(
        &self,
        keyset_id: String,
        count: u32,
    ) -> Result<(), CashuWalletError>;
    async fn get_keyset_counter(&self, keyset_id: String) -> Result<Option<u32>, CashuWalletError>;
}

// Wrapper struct for the app-facing localstore
#[derive(Clone)]
pub struct AppCashuLocalStore {
    inner: Arc<dyn CashuLocalStore>,
}

impl std::fmt::Debug for AppCashuLocalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppCashuLocalStore")
    }
}

#[async_trait]
impl WalletDatabase for AppCashuLocalStore {
    type Err = cdk::cdk_database::Error;

    // Proofs (send/receive)
    async fn get_proofs(
        &self,
        mint_url: Option<cdk_common::mint_url::MintUrl>,
        unit: Option<cdk::nuts::CurrencyUnit>,
        state: Option<Vec<cdk::nuts::State>>,
        spending_conditions: Option<Vec<cdk::nuts::SpendingConditions>>,
    ) -> Result<Vec<cdk_common::common::ProofInfo>, Self::Err> {
        let mint_url_string = mint_url.map(|url| url.to_string());
        let unit_string = unit.map(|u| u.to_string());
        let state_string = state.map(|s| serde_json::to_string(&s).unwrap_or_default());
        let spending_conditions_string =
            spending_conditions.map(|sc| serde_json::to_string(&sc).unwrap_or_default());

        let app_proofs = self
            .inner
            .get_proofs(
                mint_url_string,
                unit_string,
                state_string,
                spending_conditions_string,
            )
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;

        let mut cdk_proofs = Vec::new();
        for app_proof in app_proofs {
            let proof: cdk_common::common::ProofInfo = serde_json::from_str(&app_proof)
                .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e)))?;
            cdk_proofs.push(proof);
        }

        Ok(cdk_proofs)
    }

    async fn update_proofs(
        &self,
        added: Vec<cdk_common::common::ProofInfo>,
        removed_ys: Vec<cdk::nuts::PublicKey>,
    ) -> Result<(), Self::Err> {
        let added_strings: Vec<String> = added
            .into_iter()
            .map(|p| serde_json::to_string(&p).unwrap_or_default())
            .collect();
        let removed_strings: Vec<String> =
            removed_ys.into_iter().map(|pk| pk.to_string()).collect();

        self.inner
            .update_proofs(added_strings, removed_strings)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn update_proofs_state(
        &self,
        ys: Vec<cdk::nuts::PublicKey>,
        state: cdk::nuts::State,
    ) -> Result<(), Self::Err> {
        let ys_strings: Vec<String> = ys.into_iter().map(|pk| pk.to_string()).collect();
        let state_string = serde_json::to_string(&state).unwrap_or_default();

        self.inner
            .update_proofs_state(ys_strings, state_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    // Transaction history (send/receive)
    async fn add_transaction(
        &self,
        transaction: cdk_common::wallet::Transaction,
    ) -> Result<(), Self::Err> {
        let mut transaction_string = serde_json::to_value(&transaction).unwrap_or_default();
        transaction_string["id"] = serde_json::Value::String(transaction.id().to_string());

        self.inner
            .add_transaction(
                serde_json::to_string(&transaction_string)
                    .expect("Failed to serialize transaction"),
            )
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn get_transaction(
        &self,
        transaction_id: cdk_common::wallet::TransactionId,
    ) -> Result<Option<cdk_common::wallet::Transaction>, Self::Err> {
        let transaction_id_string = transaction_id.to_string();

        match self.inner.get_transaction(transaction_id_string).await {
            Ok(Some(transaction_string)) => serde_json::from_str(&transaction_string)
                .map(Some)
                .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e))),
            Ok(None) => Ok(None),
            Err(e) => Err(cdk::cdk_database::Error::Database(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn list_transactions(
        &self,
        mint_url: Option<cdk_common::mint_url::MintUrl>,
        direction: Option<cdk_common::wallet::TransactionDirection>,
        unit: Option<cdk::nuts::CurrencyUnit>,
    ) -> Result<Vec<cdk_common::wallet::Transaction>, Self::Err> {
        let mint_url_string = mint_url.map(|url| url.to_string());
        let direction_string = direction.map(|d| serde_json::to_string(&d).unwrap_or_default());
        let unit_string = unit.map(|u| serde_json::to_string(&u).unwrap_or_default());

        let transaction_strings = self
            .inner
            .list_transactions(mint_url_string, direction_string, unit_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;

        let mut transactions = Vec::new();
        for transaction_string in transaction_strings {
            if let Ok(transaction) = serde_json::from_str(&transaction_string) {
                transactions.push(transaction);
            }
        }
        Ok(transactions)
    }

    async fn remove_transaction(
        &self,
        transaction_id: cdk_common::wallet::TransactionId,
    ) -> Result<(), Self::Err> {
        let transaction_id_string = transaction_id.to_string();

        self.inner
            .remove_transaction(transaction_id_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn add_mint(
        &self,
        mint_url: cdk_common::mint_url::MintUrl,
        mint_info: Option<cdk::nuts::MintInfo>,
    ) -> Result<(), Self::Err> {
        let mint_url_string = mint_url.to_string();
        let mint_info_string =
            mint_info.map(|info| serde_json::to_string(&info).unwrap_or_default());

        self.inner
            .add_mint(mint_url_string, mint_info_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn remove_mint(&self, mint_url: cdk_common::mint_url::MintUrl) -> Result<(), Self::Err> {
        let mint_url_string = mint_url.to_string();

        self.inner.remove_mint(mint_url_string).await.map_err(|e| {
            cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })
    }

    async fn get_mint(
        &self,
        mint_url: cdk_common::mint_url::MintUrl,
    ) -> Result<Option<cdk::nuts::MintInfo>, Self::Err> {
        let mint_url_string = mint_url.to_string();

        match self.inner.get_mint(mint_url_string).await {
            Ok(Some(mint_info_string)) => serde_json::from_str(&mint_info_string)
                .map(Some)
                .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e))),
            Ok(None) => Ok(None),
            Err(e) => Err(cdk::cdk_database::Error::Database(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn get_mints(
        &self,
    ) -> Result<
        std::collections::HashMap<cdk_common::mint_url::MintUrl, Option<cdk::nuts::MintInfo>>,
        Self::Err,
    > {
        let mint_urls = self.inner.get_mints().await.map_err(|e| {
            cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        let mut mints = std::collections::HashMap::new();
        for mint_url_string in mint_urls {
            let mint_url =
                cdk_common::mint_url::MintUrl::from_str(&mint_url_string).map_err(|e| {
                    cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    )))
                })?;

            // Get mint info for this URL
            match self.inner.get_mint(mint_url_string).await {
                Ok(Some(mint_info_string)) => {
                    let mint_info: cdk::nuts::MintInfo = serde_json::from_str(&mint_info_string)
                        .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e)))?;
                    mints.insert(mint_url, Some(mint_info));
                }
                Ok(None) => {
                    mints.insert(mint_url, None);
                }
                Err(e) => {
                    return Err(cdk::cdk_database::Error::Database(Box::new(
                        std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
                    )));
                }
            }
        }

        Ok(mints)
    }

    async fn update_mint_url(
        &self,
        old_mint_url: cdk_common::mint_url::MintUrl,
        new_mint_url: cdk_common::mint_url::MintUrl,
    ) -> Result<(), Self::Err> {
        let old_mint_url_string = old_mint_url.to_string();
        let new_mint_url_string = new_mint_url.to_string();

        self.inner
            .update_mint_url(old_mint_url_string, new_mint_url_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn add_mint_keysets(
        &self,
        mint_url: cdk_common::mint_url::MintUrl,
        keysets: Vec<cdk::nuts::KeySetInfo>,
    ) -> Result<(), Self::Err> {
        let mint_url_string = mint_url.to_string();
        let keysets_strings: Vec<String> = keysets
            .into_iter()
            .map(|k| serde_json::to_string(&k).unwrap_or_default())
            .collect();

        self.inner
            .add_mint_keysets(mint_url_string, keysets_strings)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn get_mint_keysets(
        &self,
        mint_url: cdk_common::mint_url::MintUrl,
    ) -> Result<Option<Vec<cdk::nuts::KeySetInfo>>, Self::Err> {
        let mint_url_string = mint_url.to_string();

        match self.inner.get_mint_keysets(mint_url_string).await {
            Ok(Some(keysets_strings)) => {
                let keysets = keysets_strings
                    .iter()
                    .map(|s| serde_json::from_str(s))
                    .collect::<Result<_, _>>()
                    .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e)))?;
                Ok(Some(keysets))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(cdk::cdk_database::Error::Database(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn get_keyset_by_id(
        &self,
        keyset_id: &cdk::nuts::Id,
    ) -> Result<Option<cdk::nuts::KeySetInfo>, Self::Err> {
        let keyset_id_string = keyset_id.to_string();

        match self.inner.get_keyset_by_id(keyset_id_string).await {
            Ok(Some(keyset_string)) => serde_json::from_str(&keyset_string)
                .map(Some)
                .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e))),
            Ok(None) => Ok(None),
            Err(e) => Err(cdk::cdk_database::Error::Database(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn add_keys(&self, keyset: cdk::nuts::KeySet) -> Result<(), Self::Err> {
        let keyset_string = serde_json::to_string(&keyset).unwrap_or_default();

        self.inner.add_keys(keyset_string).await.map_err(|e| {
            cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })
    }

    async fn get_keys(&self, id: &cdk::nuts::Id) -> Result<Option<cdk::nuts::Keys>, Self::Err> {
        let id_string = id.to_string();

        match self.inner.get_keys(id_string).await {
            Ok(Some(keys_string)) => serde_json::from_str(&keys_string)
                .map(Some)
                .map_err(|e| cdk::cdk_database::Error::Database(Box::new(e))),
            Ok(None) => Ok(None),
            Err(e) => Err(cdk::cdk_database::Error::Database(Box::new(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn remove_keys(&self, id: &cdk::nuts::Id) -> Result<(), Self::Err> {
        let id_string = id.to_string();

        self.inner.remove_keys(id_string).await.map_err(|e| {
            cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })
    }

    async fn increment_keyset_counter(
        &self,
        keyset_id: &cdk::nuts::Id,
        count: u32,
    ) -> Result<(), Self::Err> {
        let keyset_id_string = keyset_id.to_string();

        self.inner
            .increment_keyset_counter(keyset_id_string, count)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn get_keyset_counter(
        &self,
        keyset_id: &cdk::nuts::Id,
    ) -> Result<Option<u32>, Self::Err> {
        let keyset_id_string = keyset_id.to_string();

        self.inner
            .get_keyset_counter(keyset_id_string)
            .await
            .map_err(|e| {
                cdk::cdk_database::Error::Database(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })
    }

    async fn add_mint_quote(&self, _quote: cdk::wallet::MintQuote) -> Result<(), Self::Err> {
        unimplemented!()
    }
    async fn get_mint_quote(
        &self,
        _quote_id: &str,
    ) -> Result<Option<cdk::wallet::MintQuote>, Self::Err> {
        unimplemented!()
    }
    async fn get_mint_quotes(&self) -> Result<Vec<cdk::wallet::MintQuote>, Self::Err> {
        unimplemented!()
    }
    async fn remove_mint_quote(&self, _quote_id: &str) -> Result<(), Self::Err> {
        unimplemented!()
    }
    async fn add_melt_quote(&self, _quote: cdk::wallet::MeltQuote) -> Result<(), Self::Err> {
        unimplemented!()
    }
    async fn get_melt_quote(
        &self,
        _quote_id: &str,
    ) -> Result<Option<cdk::wallet::MeltQuote>, Self::Err> {
        unimplemented!()
    }
    async fn remove_melt_quote(&self, _quote_id: &str) -> Result<(), Self::Err> {
        unimplemented!()
    }
}
