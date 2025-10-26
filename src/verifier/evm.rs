/// Verification module for evm network.
use crate::types::{
    ChainType, Currency, EvmChain, PaymentRequest, PaymentVerification, TransactionLog,
};
use crate::verifier::{PaymentVerifier, VerificationError};
use async_trait::async_trait;
use ethers::types::{H256, ValueOrArray};
use ethers::utils::hex;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{BlockNumber, Filter, H160, U64, U256},
};
use std::str::FromStr;
use std::sync::Arc;

/// EVM compatible blockchain payment verification module.
///
/// # Examples
///
/// ```rust
/// use x402::types::{ChainType, EvmChain};
/// use x402::verifier::evm::EvmVerifier;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let verifier = EvmVerifier::new(
///     "https://mainnet.infura.io/v3/your-key".to_string(),
///     ChainType::Evm(EvmChain::Ethereum)
/// ).await?;
/// Ok(())
/// }
/// ```
///
pub struct EvmVerifier {
    provider: Arc<Provider<Http>>,
    chain_type: ChainType,
}

impl EvmVerifier {
    pub async fn new(rpc_url: String, chain_type: ChainType) -> Result<Self, VerificationError> {
        let provider = Provider::<Http>::try_from(&rpc_url).map_err(|e| {
            VerificationError::NetworkError(format!("Failed to create provider: {}", e))
        })?;
        let provider = Arc::new(provider);
        // real chain id
        let real_chain_id = provider.get_chainid().await.map_err(|e| {
            VerificationError::NetworkError(format!("Failed to get chain ID: {}", e))
        })?;
        // get the desired chain ID from ChainType
        let expected_chain_id = match &chain_type {
            ChainType::Evm(evm_chain) => match evm_chain {
                EvmChain::Ethereum => 1,
                EvmChain::Polygon => 137,
                EvmChain::BinanceSmartChain => 56,
                EvmChain::Arbitrum => 42161,
                EvmChain::Optimism => 10,
                EvmChain::Avalanche => 43114,
                EvmChain::Base => 8453,
                EvmChain::Custom(id) => id.parse().map_err(|e| {
                    VerificationError::ParseError(format!("Invalid custom chain ID: {}", e))
                })?,
            },
            _ => return Err(VerificationError::ChainNotSupported),
        };
        if real_chain_id.as_u64() != expected_chain_id {
            return Err(VerificationError::NetworkError(format!(
                "Chain ID mismatch: expected {}, got {}",
                expected_chain_id, real_chain_id
            )));
        }
        Ok(Self {
            provider,
            chain_type,
        })
    }

    async fn verify_payment_internal(
        &self,
        payment_request: &PaymentRequest,
        payer_address: &str,
    ) -> Result<PaymentVerification, VerificationError> {
        let payer = Self::parse_address(payer_address)?;
        let recipient = Self::parse_address(&payment_request.recipient)?;
        let required_amount = Self::parse_amount(&payment_request.amount)?;
        let (is_paid, transaction_logs) = match &payment_request.currency {
            Currency::Native => {
                self.verify_native_payment(payer, recipient, required_amount)
                    .await?
            }
            Currency::Token { address, decimals } => {
                let token_address = Self::parse_address(address)?;
                self.verify_erc20_payment(
                    payer,
                    recipient,
                    token_address,
                    required_amount,
                    *decimals,
                )
                .await?
            }
        };
        Ok(PaymentVerification {
            is_paid,
            paid_amount: if is_paid {
                payment_request.amount.clone()
            } else {
                "0".to_string()
            },
            transaction_hash: transaction_logs
                .first()
                .map(|log| log.transaction_hash.clone()),
            verified_at: Self::current_timestamp(),
            chain: payment_request.chain.clone(),
            transaction_logs,
        })
    }

    async fn verify_native_payment(
        &self,
        payer: H160,
        recipient: H160,
        required_amount: U256,
    ) -> Result<(bool, Vec<TransactionLog>), VerificationError> {
        self.check_recent_transactions(payer, recipient, required_amount)
            .await
    }

    async fn verify_erc20_payment(
        &self,
        payer: H160,
        recipient: H160,
        token_address: H160,
        required_amount: U256,
        decimals: u8,
    ) -> Result<(bool, Vec<TransactionLog>), VerificationError> {
        let adjusted_amount = required_amount * U256::from(10).pow(U256::from(decimals));
        // search ERC20 Transfer events
        let filter = self
            .create_erc20_transfer_filter(payer, recipient, token_address)
            .await?;
        let logs =
            self.provider.get_logs(&filter).await.map_err(|e| {
                VerificationError::RpcError(format!("Failed to get ERC20 logs: {}", e))
            })?;
        let mut found_payment = false;
        let mut transaction_logs = Vec::new();
        for log in logs {
            if let (Some(tx_hash), Some(data)) = (log.transaction_hash, log.data.get(0..32)) {
                let amount = U256::from_big_endian(data);
                let log_entry = TransactionLog {
                    transaction_hash: format!("{:?}", tx_hash),
                    from: format!("{:?}", payer),
                    to: format!("{:?}", recipient),
                    value: amount.to_string(),
                    block_number: log.block_number.unwrap_or_default().as_u64(),
                    log_index: log.log_index.unwrap_or_default().as_u64(),
                    data: Some(hex::encode(data)),
                };
                transaction_logs.push(log_entry);
                if amount >= adjusted_amount {
                    found_payment = true;
                }
            }
        }
        Ok((found_payment, transaction_logs))
    }

    async fn check_recent_transactions(
        &self,
        payer: H160,
        recipient: H160,
        required_amount: U256,
    ) -> Result<(bool, Vec<TransactionLog>), VerificationError> {
        let latest_block = self.provider.get_block_number().await.map_err(|e| {
            VerificationError::RpcError(format!("Failed to get block number: {}", e))
        })?;
        let from_block = latest_block
            .checked_sub(U64::from(100))
            .unwrap_or(U64::zero());
        let filter = Filter::new()
            .from_block(BlockNumber::Number(from_block))
            .to_block(BlockNumber::Number(latest_block))
            .address(recipient);
        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| VerificationError::RpcError(format!("Rpc Error: {:?}", e)))?;
        let mut found_payment = false;
        let mut transaction_logs = Vec::new();
        for log in logs {
            if let Some(tx_hash) = log.transaction_hash {
                if let Ok(Some(tx)) = self.provider.get_transaction(tx_hash).await {
                    let log_entry = TransactionLog {
                        transaction_hash: format!("{:?}", tx_hash),
                        from: format!("{:?}", tx.from),
                        to: format!("{:?}", tx.to.unwrap_or_default()),
                        value: tx.value.to_string(),
                        block_number: log.block_number.unwrap_or_default().as_u64(),
                        log_index: log.log_index.unwrap_or_default().as_u64(),
                        data: None,
                    };
                    transaction_logs.push(log_entry);
                    if tx.from == payer && tx.value >= required_amount {
                        found_payment = true;
                    }
                }
            }
        }
        Ok((found_payment, transaction_logs))
    }

    async fn create_erc20_transfer_filter(
        &self,
        from: H160,
        to: H160,
        token_address: H160,
    ) -> Result<Filter, VerificationError> {
        let latest_block = self.provider.get_block_number().await.map_err(|e| {
            VerificationError::RpcError(format!("Failed to get block number: {:?}", e))
        })?;
        let from_block = latest_block
            .checked_sub(U64::from(100))
            .unwrap_or(U64::zero());
        let filter = Filter::new()
            .from_block(BlockNumber::Number(from_block))
            .to_block(BlockNumber::Number(latest_block))
            .address(token_address)
            .event("Transfer(address,address,uint256)")
            .topic1(ValueOrArray::Value(H256::from(from)))
            .topic2(ValueOrArray::Value(H256::from(to)));
        Ok(filter)
    }

    /// parse address
    fn parse_address(address: &str) -> Result<H160, VerificationError> {
        H160::from_str(address).map_err(|_| VerificationError::InvalidAddress)
    }

    /// parse amount
    fn parse_amount(amount: &str) -> Result<U256, VerificationError> {
        U256::from_dec_str(amount)
            .map_err(|e| VerificationError::ParseError(format!("Parse Error: {:?}", e)))
    }

    /// current timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[async_trait]
impl PaymentVerifier for EvmVerifier {
    async fn verify_payment(
        &self,
        payment_request: &PaymentRequest,
        payer_address: &str,
    ) -> Result<PaymentVerification, VerificationError> {
        self.verify_payment_internal(payment_request, payer_address)
            .await
    }

    fn supports_chain(&self, chain_type: &ChainType) -> bool {
        matches!(chain_type, ChainType::Evm(_))
    }
}
