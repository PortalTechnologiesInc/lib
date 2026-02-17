
use axum::async_trait;
use breez_sdk_spark::{
    BreezSdk, ConnectRequest, GetInfoRequest, ListPaymentsRequest, Network, PaymentDetails, PaymentStatus, PaymentType,
    PrepareSendPaymentRequest, ReceivePaymentMethod, ReceivePaymentRequest, SendPaymentMethod,
    SendPaymentRequest, Seed, connect, default_config
};

use tracing::info;

use crate::{PortalWallet, PortalWalletError, Result};

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
        // Optionally set the expiry duration in seconds
        let optional_expiry_secs = Some(3600_u32);
        let receive_response = self
            .sdk
            .receive_payment(ReceivePaymentRequest {
                payment_method: ReceivePaymentMethod::Bolt11Invoice {
                    description,
                    amount_sats: Some(sats / 1000),
                    expiry_secs: optional_expiry_secs,
                },
            })
            .await?;

        Ok(receive_response.payment_request)
    }

    async fn pay_invoice(&self, invoice: String) -> Result<String> {
        let prepare_response = self
            .sdk
            .prepare_send_payment(PrepareSendPaymentRequest {
                payment_request: invoice,
                amount: None,
                token_identifier: None,
                conversion_options: None,
            })
            .await?;

        // Check fee acceptability before sending.
        // Policy:
        //   - Below 500 sats: accept any fee (small payments are exempt).
        //   - 500 sats or above: reject if fees exceed max(1% of amount, 1000 sats).
        //   - Zero-amount invoices: reject if fees exceed 1000 sats.
        // This guards against unexpectedly high fees in programmatic payments where
        // there is no interactive user to approve.
        if let SendPaymentMethod::Bolt11Invoice {
            invoice_details,
            spark_transfer_fee_sats,
            lightning_fee_sats,
        } = &prepare_response.payment_method
        {
            let spark_fee = spark_transfer_fee_sats.unwrap_or(0);
            let total_fee = lightning_fee_sats + spark_fee;

            info!("Lightning fees: {lightning_fee_sats} sats");
            info!("Spark transfer fees: {spark_fee} sats");

            // Use invoice amount (in msats) to compute a percentage-based cap.
            if let Some(amount_msat) = invoice_details.amount_msat {
                let amount_sats = amount_msat / 1000;

                if amount_sats < 500 {
                    // Small payments (below 500 sats): accept any fee.
                    info!("Total fees: {total_fee} sats (small payment {amount_sats} sats — fee check skipped)");
                } else {
                    // 500 sats or above: max acceptable fee = max(1% of amount, 1000 sats).
                    let one_percent = amount_sats / 100;
                    let max_fee = std::cmp::max(one_percent, 1000);

                    info!("Total fees: {total_fee} sats (max allowed: {max_fee} sats, amount: {amount_sats} sats)");

                    if total_fee > max_fee {
                        return Err(PortalWalletError::FeeTooHigh(format!(
                            "{total_fee} sats exceeds maximum of {max_fee} sats \
                             (payment amount: {amount_sats} sats)"
                        )));
                    }
                }
            } else {
                // No amount in invoice (zero-amount invoice); apply absolute cap of 1000 sats.
                info!("Total fees: {total_fee} sats (max allowed: 1000 sats, zero-amount invoice)");

                if total_fee > 1000 {
                    return Err(PortalWalletError::FeeTooHigh(format!(
                        "{total_fee} sats exceeds absolute maximum of 1000 sats \
                         (zero-amount invoice)"
                    )));
                }
            }
        }

        let send_response = self
            .sdk
            .send_payment(SendPaymentRequest {
                prepare_response,
                options: None,
                idempotency_key: None,
            })
            .await?;

        // Extract preimage from payment details if available
        let preimage = send_response
            .payment
            .details
            .and_then(|d| match d {
                PaymentDetails::Lightning { preimage, .. } => preimage,
                _ => None,
            })
            .unwrap_or_default();

        Ok(preimage)
    }

    async fn is_invoice_paid(&self, invoice: String) -> Result<(bool, Option<String>)> {
        let batch_size = 100;
        let mut offset = 0;

        loop {
            let list_payments_response = self
                .sdk
                .list_payments(ListPaymentsRequest {
                    // Filter by payment type
                    type_filter: Some(vec![PaymentType::Receive]),
                    // Filter by status
                    status_filter: Some(vec![PaymentStatus::Completed]),
                    asset_filter: None,
                    // Time range filters
                    from_timestamp: None, // Unix timestamp
                    to_timestamp: None,   // Unix timestamp
                    // Pagination
                    limit: Some(batch_size),
                    offset: Some(offset),
                    // Sort order (true = oldest first, false = newest first)
                    sort_ascending: Some(false),
                    payment_details_filter: None,
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

    async fn get_balance(&self) -> Result<u64> {
        let balance = self.sdk.get_info(GetInfoRequest {
            ensure_synced: Some(true),
        })
        .await?;
        Ok(balance.balance_sats * 1000)
    }
}
