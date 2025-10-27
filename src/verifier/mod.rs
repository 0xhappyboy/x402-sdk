use crate::types::{ChainType, PaymentRequest, PaymentVerification};
use async_trait::async_trait;
use std::collections::HashMap;

pub mod evm;
pub mod solana;

#[derive(Debug)]
pub enum VerificationError {
    NetworkError(String),
    InvalidAddress,
    ChainNotSupported,
    RpcError(String),
    TransactionNotFound,
    InsufficientAmount,
    InvalidCurrency,
    Timeout,
    ParseError(String),
    Error(String),
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::ChainNotSupported => write!(f, "Chain not supported"),
            Self::RpcError(msg) => write!(f, "RPC error: {}", msg),
            Self::TransactionNotFound => write!(f, "Transaction not found"),
            Self::InsufficientAmount => write!(f, "Insufficient payment amount"),
            Self::InvalidCurrency => write!(f, "Invalid currency"),
            Self::Timeout => write!(f, "Verification timeout"),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for VerificationError {}

#[async_trait]
pub trait PaymentVerifier: Send + Sync {
    async fn verify_payment(
        &self,
        payment_request: &PaymentRequest,
        payer_address: &str,
    ) -> Result<PaymentVerification, VerificationError>;

    fn supports_chain(&self, chain_type: &ChainType) -> bool;
}

pub struct VerifierRegistry {
    verifiers: HashMap<ChainType, Box<dyn PaymentVerifier>>,
}

impl VerifierRegistry {
    pub fn new() -> Self {
        Self {
            verifiers: HashMap::new(),
        }
    }

    pub fn register_verifier(&mut self, chain_type: ChainType, verifier: Box<dyn PaymentVerifier>) {
        self.verifiers.insert(chain_type, verifier);
    }

    pub fn get_verifier(&self, chain_type: &ChainType) -> Option<&dyn PaymentVerifier> {
        self.verifiers.get(chain_type).map(|v| v.as_ref())
    }

    pub fn has_verifier(&self, chain_type: &ChainType) -> bool {
        self.verifiers.contains_key(chain_type)
    }

    pub fn supported_chains(&self) -> Vec<ChainType> {
        self.verifiers.keys().cloned().collect()
    }

    pub fn remove_verifier(&mut self, chain_type: &ChainType) -> Option<Box<dyn PaymentVerifier>> {
        self.verifiers.remove(chain_type)
    }
}

impl Default for VerifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}
