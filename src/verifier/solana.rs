use crate::types::{ChainType, PaymentRequest, PaymentVerification, TransactionLog};
use crate::verifier::{PaymentVerifier, VerificationError};
use async_trait::async_trait;
use solana_network_sdk::Solana;
use solana_network_sdk::tool::address::is_valid_address;
use solana_network_sdk::trade::TransactionInfo;
use solana_network_sdk::types::Mode;
use std::sync::Arc;

pub struct SolanaVerifier {
    client: Arc<Solana>,
}

impl SolanaVerifier {
    pub fn new() -> Self {
        let client = Solana::new(Mode::MAIN).unwrap();
        Self {
            client: Arc::new(client),
        }
    }

    /// check whether a single transaction meets the payment conditions
    fn check_transaction_payment(
        &self,
        transaction: &TransactionInfo,
        recipient: &str,
        required_amount: &str,
    ) -> Result<bool, VerificationError> {
        // check if the transaction status is successful
        if !transaction.is_successful() {
            return Ok(false);
        }
        // check if the payment address matches
        if !transaction.is_recipient(recipient) {
            return Ok(false);
        }
        // parse the required amount (supports SOL and Lamports formats)
        let required_lamports = Self::parse_amount_to_lamports(required_amount)
            .map_err(|e| VerificationError::ParseError(e))?;
        // check whether the payment amount meets the requirements
        let paid_lamports = transaction.get_payment_amount();
        if paid_lamports >= required_lamports {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Parse amount string into lamports
    fn parse_amount_to_lamports(amount: &str) -> Result<u64, String> {
        const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;
        let amount = amount.trim().replace(',', "");
        if amount.is_empty() {
            return Err("Amount cannot be empty".to_string());
        }
        if amount.contains('.') {
            let sol_amount: f64 = amount
                .parse()
                .map_err(|_| format!("Invalid SOL amount format: {}", amount))?;

            if sol_amount < 0.0 {
                return Err("The amount cannot be negative".to_string());
            }
            let lamports = (sol_amount * LAMPORTS_PER_SOL).round() as u64;
            Ok(lamports)
        } else {
            let lamports: u64 = amount
                .parse()
                .map_err(|_| format!("Invalid lamports amount format: {}", amount))?;
            Ok(lamports)
        }
    }
}

#[async_trait]
impl PaymentVerifier for SolanaVerifier {
    async fn verify_payment(
        &self,
        payment_request: &PaymentRequest,
        payer_address: &str,
    ) -> Result<PaymentVerification, VerificationError> {
        if !is_valid_address(payer_address) {
            return Err(VerificationError::Error("payer address error".to_string()));
        }
        if !is_valid_address(&payment_request.recipient) {
            return Err(VerificationError::Error(
                "recipient address error".to_string(),
            ));
        }
        let trade = self.client.create_trade();
        let transactions = trade
            .get_transactions_by_recipient_and_payer_strict(
                &payment_request.recipient,
                payer_address,
                50,
            )
            .await;
        let mut found_payment = false;
        let mut transaction_logs = Vec::new();
        let mut paid_amount = "0".to_string();
        let mut transaction_hash = None;
        match transactions {
            Ok(transactions) => {
                for transaction in transactions {
                    let transaction_info = TransactionInfo::from_encoded_transaction(
                        &trade
                            .get_transaction_details(&transaction.signature)
                            .await
                            .unwrap(),
                        &transaction.signature,
                        "solana",
                    );
                    if self.check_transaction_payment(
                        &transaction_info,
                        &payment_request.recipient,
                        &payment_request.amount,
                    )? {
                        found_payment = true;
                        paid_amount = transaction_info.output_amount.unwrap_or(0).to_string();
                        transaction_hash = Some(transaction.signature.clone());
                        transaction_logs.push(TransactionLog {
                            transaction_hash: transaction_info.transaction_hash,
                            from: transaction_info.from,
                            to: transaction_info.to,
                            value: transaction_info.value,
                            block_number: transaction_info.block_number,
                            log_index: transaction_info.log_index,
                            data: transaction_info.data,
                        });
                        break;
                    }
                }
            }
            Err(_) => todo!(),
        }
        Ok(PaymentVerification {
            is_paid: found_payment,
            paid_amount,
            transaction_hash,
            verified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chain: payment_request.chain.clone(),
            transaction_logs,
        })
    }

    fn supports_chain(&self, chain_type: &ChainType) -> bool {
        matches!(chain_type, ChainType::Solana(_))
    }
}
