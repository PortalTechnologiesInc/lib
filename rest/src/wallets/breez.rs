use std::env;

use axum::async_trait;
use breez_sdk_spark::{
    connect, default_config, BreezSdk, ConnectRequest, ListPaymentsRequest, Network,
    PaymentDetails, PaymentStatus, ReceivePaymentMethod, ReceivePaymentRequest, Seed,
};

use crate::wallets::PortalWallet;

pub struct BreezSparkWallet {
    sdk: BreezSdk,
}

impl BreezSparkWallet {
    pub async fn new(mnemonic: String) -> anyhow::Result<Self> {
        let api_key =
            env::var("BREEZ_API_KEY").expect("BREEZ_API_KEY environment variable is required");
        let breez_storage_dir = env::var("BREEZ_STORAGE_DIR")
            .expect("BREEZ_STORAGE_DIR environment variable is required");
        let seed = Seed::Mnemonic {
            mnemonic,
            passphrase: None,
        };

        let mut config = default_config(Network::Mainnet);
        config.api_key = Some(api_key);

        let sdk = connect(ConnectRequest {
            config,
            seed,
            storage_dir: breez_storage_dir,
        })
        .await?;

        Ok(Self { sdk })
    }
}

#[async_trait]
impl PortalWallet for BreezSparkWallet {
    async fn make_invoice(&self, sats: u64, description: Option<String>) -> anyhow::Result<String> {
        let description = description.unwrap_or("Portal invoice".into());

        let receive_response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                payment_method: ReceivePaymentMethod::Bolt11Invoice {
                    description,
                    amount_sats: Some(sats),
                },
            })
            .await?;

        Ok(receive_response.payment_request)
    }

    async fn is_invoice_paid(&self, invoice: String) -> anyhow::Result<(bool, Option<String>)> {
        let list_payments_response = self
            .sdk
            .list_payments(ListPaymentsRequest {
                limit: None,
                offset: None,
            })
            .await?;

        for payment in list_payments_response.payments {
            match payment.details {
                Some(PaymentDetails::Lightning {
                    invoice: payment_invoice,
                    preimage,
                    ..
                }) => {
                    if invoice == payment_invoice {
                        return Ok((payment.status == PaymentStatus::Completed, preimage));
                    }
                }
                _ => {}
            }
        }

        Ok((false, None))
    }
}
