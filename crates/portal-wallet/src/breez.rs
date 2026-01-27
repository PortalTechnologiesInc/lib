use std::env;

use axum::async_trait;
use breez_sdk_spark::{
    BreezSdk, ConnectRequest, ListPaymentsRequest, Network, PaymentDetails, PaymentStatus,
    ReceivePaymentMethod, ReceivePaymentRequest, Seed, connect, default_config,
};

use crate::{PortalWallet, Result};

/// Breez Spark Wallet implementation
pub struct BreezSparkWallet {
    sdk: BreezSdk,
}

impl BreezSparkWallet {
    pub async fn new(api_key: String, storage_dir: String, mnemonic: String) -> Result<Self> {
        let seed = Seed::Mnemonic {
            mnemonic,
            passphrase: None,
        };

        let mut config = default_config(Network::Mainnet);
        config.api_key = Some(api_key);

        let sdk = connect(ConnectRequest {
            config,
            seed,
            storage_dir,
        })
        .await?;

        Ok(Self { sdk })
    }
}

#[async_trait]
impl PortalWallet for BreezSparkWallet {
    async fn make_invoice(&self, sats: u64, description: Option<String>) -> Result<String> {
        let description = description.unwrap_or("Portal invoice".into());

        let receive_response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                payment_method: ReceivePaymentMethod::Bolt11Invoice {
                    description,
                    amount_sats: Some(sats / 1000),
                },
            })
            .await?;

        Ok(receive_response.payment_request)
    }

    async fn is_invoice_paid(&self, invoice: String) -> Result<(bool, Option<String>)> {
        let batch_size = 100;
        let mut offset = 0;

        loop {
            let list_payments_response = self
                .sdk
                .list_payments(ListPaymentsRequest {
                    limit: Some(batch_size),
                    offset: Some(offset),
                })
                .await?;

            if list_payments_response.payments.len() == 0 {
                break;
            }

            for payment in list_payments_response.payments.iter() {
                match &payment.details {
                    Some(PaymentDetails::Lightning {
                        invoice: payment_invoice,
                        preimage,
                        ..
                    }) => {
                        if invoice == *payment_invoice {
                            return Ok((
                                payment.status == PaymentStatus::Completed,
                                preimage.clone(),
                            ));
                        }
                    }
                    _ => {}
                }
            }

            if list_payments_response.payments.len() < batch_size as usize {
                break;
            }

            offset += batch_size;
        }

        Ok((false, None))
    }
}
