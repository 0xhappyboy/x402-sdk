/// Type definitions for global use.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChainType {
    Evm(EvmChain),
    Aptos(AptosChain),
    Sui(SuiChain),
    Solana(SolanaChain),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EvmChain {
    Ethereum,
    Polygon,
    BinanceSmartChain,
    Arbitrum,
    Optimism,
    Avalanche,
    Base,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AptosChain {
    Mainnet,
    Testnet,
    Devnet,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SuiChain {
    Mainnet,
    Testnet,
    Devnet,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SolanaChain {
    Mainnet,
    Testnet,
    Devnet,
    Custom(String),
}

impl ChainType {
    pub fn get_standard_chain_id(&self) -> String {
        match self {
            ChainType::Evm(evm_chain) => match evm_chain {
                EvmChain::Ethereum => "1",
                EvmChain::Polygon => "137",
                EvmChain::BinanceSmartChain => "56",
                EvmChain::Arbitrum => "42161",
                EvmChain::Optimism => "10",
                EvmChain::Avalanche => "43114",
                EvmChain::Base => "8453",
                EvmChain::Custom(id) => id,
            }
            .to_string(),
            ChainType::Aptos(aptos_chain) => match aptos_chain {
                AptosChain::Mainnet => "1",
                AptosChain::Testnet => "2",
                AptosChain::Devnet => "devnet",
                AptosChain::Custom(id) => id,
            }
            .to_string(),
            ChainType::Sui(sui_chain) => match sui_chain {
                SuiChain::Mainnet => "mainnet",
                SuiChain::Testnet => "testnet",
                SuiChain::Devnet => "devnet",
                SuiChain::Custom(id) => id,
            }
            .to_string(),
            ChainType::Solana(solana_chain) => match solana_chain {
                SolanaChain::Mainnet => "mainnet-beta",
                SolanaChain::Testnet => "testnet",
                SolanaChain::Devnet => "devnet",
                SolanaChain::Custom(id) => id,
            }
            .to_string(),
            ChainType::Custom(id) => id.clone(),
        }
    }

    pub fn get_display_name(&self) -> String {
        match self {
            ChainType::Evm(evm_chain) => match evm_chain {
                EvmChain::Ethereum => "Ethereum",
                EvmChain::Polygon => "Polygon",
                EvmChain::BinanceSmartChain => "BNB Smart Chain",
                EvmChain::Arbitrum => "Arbitrum",
                EvmChain::Optimism => "Optimism",
                EvmChain::Avalanche => "Avalanche",
                EvmChain::Base => "Base",
                EvmChain::Custom(name) => name,
            }
            .to_string(),
            ChainType::Aptos(aptos_chain) => match aptos_chain {
                AptosChain::Mainnet => "Aptos Mainnet",
                AptosChain::Testnet => "Aptos Testnet",
                AptosChain::Devnet => "Aptos Devnet",
                AptosChain::Custom(name) => name,
            }
            .to_string(),
            ChainType::Sui(sui_chain) => match sui_chain {
                SuiChain::Mainnet => "Sui Mainnet",
                SuiChain::Testnet => "Sui Testnet",
                SuiChain::Devnet => "Sui Devnet",
                SuiChain::Custom(name) => name,
            }
            .to_string(),
            ChainType::Solana(solana_chain) => match solana_chain {
                SolanaChain::Mainnet => "Solana Mainnet",
                SolanaChain::Testnet => "Solana Testnet",
                SolanaChain::Devnet => "Solana Devnet",
                SolanaChain::Custom(name) => name,
            }
            .to_string(),
            ChainType::Custom(name) => name.clone(),
        }
    }

    pub fn is_evm(&self) -> bool {
        matches!(self, ChainType::Evm(_))
    }

    pub fn is_aptos(&self) -> bool {
        matches!(self, ChainType::Aptos(_))
    }

    pub fn is_sui(&self) -> bool {
        matches!(self, ChainType::Sui(_))
    }

    pub fn is_solana(&self) -> bool {
        matches!(self, ChainType::Solana(_))
    }
}

impl ChainType {
    pub fn ethereum() -> Self {
        ChainType::Evm(EvmChain::Ethereum)
    }

    pub fn polygon() -> Self {
        ChainType::Evm(EvmChain::Polygon)
    }

    pub fn bsc() -> Self {
        ChainType::Evm(EvmChain::BinanceSmartChain)
    }

    pub fn aptos_mainnet() -> Self {
        ChainType::Aptos(AptosChain::Mainnet)
    }

    pub fn sui_mainnet() -> Self {
        ChainType::Sui(SuiChain::Mainnet)
    }

    pub fn solana_mainnet() -> Self {
        ChainType::Solana(SolanaChain::Mainnet)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_type: ChainType,
    pub chain_id: String,
    pub rpc_url: Option<String>,
}

impl ChainConfig {
    pub fn new(chain_type: ChainType, rpc_url: Option<String>) -> Self {
        let chain_id = chain_type.get_standard_chain_id();
        Self {
            chain_type,
            chain_id,
            rpc_url,
        }
    }

    pub fn from_chain_type(chain_type: ChainType) -> Self {
        Self::new(chain_type, None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub amount: String,
    pub currency: Currency,
    pub recipient: String,
    pub chain: ChainConfig,
    pub description: Option<String>,
    pub expires_at: Option<u64>,
    pub nonce: String,
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
    pub transaction_logs: Vec<TransactionLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402ProtocolResponse {
    pub status: u16,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLog {
    pub transaction_hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub block_number: u64,
    pub log_index: u64,
    pub data: Option<String>,
}
