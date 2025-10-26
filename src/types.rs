/// This module defines types for global use.
use serde::{Deserialize, Serialize};

/// based on http protocol payment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub amount: String, // amount, the string format is used to avoid loss of precision.
    pub currency: Currency,
    pub recipient: String, // recipient address
    pub chain: ChainConfig,
    pub description: Option<String>,
    pub expires_at: Option<u64>,
    pub nonce: String, // nonce, anti-replay attack
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_type: ChainType,
    pub chain_id: String,
    pub rpc_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChainType {
    Evm,
    Aptos,
    Sui,
    Solana,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Currency {
    Native,
    Token { address: String, decimals: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentVerification {
    pub is_paid: bool,
    pub paid_amount: String,
    pub transaction_hash: Option<String>,
    pub verified_at: u64,
    pub chain: ChainConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402ProtocolResponse {
    pub status: u16, // 402, only handle HTTP 402 status
    pub payment_required: PaymentRequest,
    pub verification_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub should_serve_content: bool,
    pub http_status: u16,
    pub x402_response: Option<X402ProtocolResponse>,
    pub verification: Option<PaymentVerification>,
}

#[derive(Debug, Clone)]
pub struct X402Config {
    pub default_chain: ChainConfig,
    pub service_address: String,
    pub service_fee: String,
    pub cache_ttl: u64,
}
