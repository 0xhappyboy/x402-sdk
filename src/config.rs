/// Configuration module
use crate::types::{AptosChain, ChainConfig, ChainType, EvmChain, SolanaChain, SuiChain};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum ConfigError {
    FileNotFound(String),
    InvalidConfig(String),
    ChainMissing(ChainType),
    IoError(String),
    SerializationError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {}", path),
            ConfigError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            ConfigError::ChainMissing(chain) => {
                write!(f, "Chain configuration missing: {:?}", chain)
            }
            ConfigError::IoError(err) => write!(f, "IO error: {}", err),
            ConfigError::SerializationError(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::SerializationError(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Config {
    pub service: ServiceConfig,
    pub chains: HashMap<ChainType, ChainConfig>,
    pub payments: PaymentConfig,
    pub cache: CacheConfig,
    pub default_chain: ChainType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub description: String,
    pub base_verification_url: String,
    pub default_currency: CurrencyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConfig {
    pub currency_type: CurrencyType,
    pub address: Option<String>,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurrencyType {
    Native,
    Erc20,
    Erc721,
    Coin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    pub default_amount: String,
    pub expiration_time_secs: u64,
    pub allowed_currencies: Vec<CurrencyConfig>,
    pub fee_recovery_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: usize,
}

pub struct ConfigManager {
    config: X402Config,
    environment: HashMap<String, String>,
}

impl ConfigManager {
    pub fn new() -> Result<Self, ConfigError> {
        let default_config = Self::default_config();
        let environment = Self::load_environment_variables();

        Ok(Self {
            config: default_config,
            environment,
        })
    }

    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| ConfigError::FileNotFound(path.to_string()))?;

        let config: X402Config = serde_json::from_str(&content)?;
        let environment = Self::load_environment_variables();

        Ok(Self {
            config,
            environment,
        })
    }

    pub fn from_config(config: X402Config) -> Self {
        let environment = Self::load_environment_variables();
        Self {
            config,
            environment,
        }
    }

    pub fn get_config(&self) -> &X402Config {
        &self.config
    }

    pub fn get_chain_config(&self, chain_type: &ChainType) -> Option<&ChainConfig> {
        self.config.chains.get(chain_type)
    }

    pub fn get_default_chain_config(&self) -> Result<&ChainConfig, ConfigError> {
        self.config
            .chains
            .get(&self.config.default_chain)
            .ok_or_else(|| ConfigError::ChainMissing(self.config.default_chain.clone()))
    }

    pub fn get_service_address(&self) -> String {
        self.environment
            .get("X402_SERVICE_ADDRESS")
            .cloned()
            .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    }

    pub fn update_config<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut X402Config),
    {
        updater(&mut self.config);
    }

    fn load_environment_variables() -> HashMap<String, String> {
        std::env::vars()
            .filter(|(key, _)| key.starts_with("X402_") || key.starts_with("RPC_"))
            .collect()
    }

    fn default_config() -> X402Config {
        X402Config {
            service: ServiceConfig {
                name: "X402 Payment Service".to_string(),
                description: "A service protected by x402 payment protocol".to_string(),
                base_verification_url: "https://api.example.com/verify".to_string(),
                default_currency: CurrencyConfig {
                    currency_type: CurrencyType::Native,
                    address: None,
                    decimals: 18,
                },
            },
            chains: HashMap::from([
                (
                    ChainType::Evm(EvmChain::Ethereum),
                    ChainConfig::new(
                        ChainType::Evm(EvmChain::Ethereum),
                        Some("https://eth.llamarpc.com".to_string()),
                    ),
                ),
                (
                    ChainType::Evm(EvmChain::Polygon),
                    ChainConfig::new(
                        ChainType::Evm(EvmChain::Polygon),
                        Some("https://polygon-rpc.com".to_string()),
                    ),
                ),
            ]),
            payments: PaymentConfig {
                default_amount: "1000000000000000".to_string(),
                expiration_time_secs: 3600,
                allowed_currencies: vec![CurrencyConfig {
                    currency_type: CurrencyType::Native,
                    address: None,
                    decimals: 18,
                }],
                fee_recovery_percent: 0.1,
            },
            cache: CacheConfig {
                enabled: true,
                ttl_secs: 300,
                max_entries: 1000,
            },
            default_chain: ChainType::Evm(EvmChain::Ethereum),
        }
    }
}

pub struct ConfigBuilder {
    config: X402Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ConfigManager::default_config(),
        }
    }

    pub fn with_service_name(mut self, name: &str) -> Self {
        self.config.service.name = name.to_string();
        self
    }

    pub fn with_default_chain(mut self, chain_type: ChainType) -> Self {
        self.config.default_chain = chain_type;
        self
    }

    pub fn with_chain(mut self, chain_type: ChainType, chain_config: ChainConfig) -> Self {
        self.config.chains.insert(chain_type, chain_config);
        self
    }

    pub fn with_payment_amount(mut self, amount: &str) -> Self {
        self.config.payments.default_amount = amount.to_string();
        self
    }

    pub fn with_expiration_time(mut self, seconds: u64) -> Self {
        self.config.payments.expiration_time_secs = seconds;
        self
    }

    pub fn build(self) -> X402Config {
        self.config
    }
}
