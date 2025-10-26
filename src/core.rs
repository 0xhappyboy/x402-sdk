/// x402 Core module.
use crate::config::{ConfigError, ConfigManager};
use crate::types::{
    ChainType, Currency, PaymentRequest, PaymentVerification, VerificationResult,
    X402ProtocolResponse,
};
use crate::verifier::{PaymentVerifier, VerificationError, VerifierRegistry};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Core for handling x402 Payment Required protocol.
///
/// # Examples
///
/// ```rust
/// use x402::core::X402;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut engine = X402::from_config_file("config.toml")?;
///
/// engine.register_chain_verifier(
///     ChainType::Evm(EvmChain::Ethereum),
///     "https://mainnet.infura.io/v3/your-key".to_string()
/// ).await?;
///
/// // Handle access request
/// let result = engine.handle_access_request(
///     "0x742E4D6c9Ff68c6E355B069E2775D3Dd6876b4a5",
///     "/premium/content",
///     None,
///     None
/// ).await?;
///
/// if result.should_serve_content {
///     // Return 402 Payment Required with payment details
///     println!("Payment required: {:?}", result.x402_response);
/// }
/// # Ok(())
/// # }
/// ```
pub struct X402 {
    config_manager: ConfigManager,
    verifier_registry: VerifierRegistry,
    payment_sessions_cache: Arc<RwLock<HashMap<String, PaymentSession>>>,
}

impl X402 {
    pub fn new(config_manager: ConfigManager) -> Result<Self, EngineError> {
        Ok(Self {
            config_manager,
            verifier_registry: VerifierRegistry::new(),
            payment_sessions_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn from_config_file(path: &str) -> Result<Self, EngineError> {
        let config_manager = ConfigManager::from_file(path)?;
        Self::new(config_manager)
    }

    pub fn from_default_config() -> Result<Self, EngineError> {
        let config_manager = ConfigManager::new()?;
        Self::new(config_manager)
    }

    /// register chain verifier by chain type
    pub async fn register_chain_verifier(
        &mut self,
        chain_type: ChainType,
        rpc_url: String,
    ) -> Result<(), EngineError> {
        let chain_config = self
            .config_manager
            .get_chain_config(&chain_type)
            .ok_or_else(|| EngineError::ChainNotSupported(chain_type.clone()))?;
        let verifier: Box<dyn PaymentVerifier> = match &chain_type {
            ChainType::Evm(_) => {
                use crate::verifier::evm::EvmVerifier;
                let evm_verifier = EvmVerifier::new(rpc_url, chain_type.clone())
                    .await
                    .map_err(EngineError::VerificationError)?;
                Box::new(evm_verifier)
            }
            ChainType::Aptos(_) => {
                use crate::verifier::aptos::AptosVerifier;
                let aptos_verifier = AptosVerifier::new(rpc_url, chain_type.clone());
                Box::new(aptos_verifier)
            }
            ChainType::Sui(_) => {
                use crate::verifier::sui::SuiVerifier;
                let sui_verifier = SuiVerifier::new(rpc_url, chain_type.clone());
                Box::new(sui_verifier)
            }
            ChainType::Solana(_) => {
                use crate::verifier::solana::SolanaVerifier;
                let solana_verifier = SolanaVerifier::new(rpc_url, chain_type.clone());
                Box::new(solana_verifier)
            }
            ChainType::Custom(_) => {
                return Err(EngineError::ChainNotSupported(chain_type));
            }
        };
        self.verifier_registry
            .register_verifier(chain_type, verifier);
        Ok(())
    }

    pub async fn verify_payment(
        &self,
        user_address: &str,
        payment_nonce: &str,
    ) -> Result<PaymentVerification, EngineError> {
        let (chain_type, payment_request) = {
            let sessions = self.payment_sessions_cache.read().unwrap();
            let session = sessions
                .get(payment_nonce)
                .ok_or(EngineError::InvalidSession)?;

            if session.user_address != user_address {
                return Err(EngineError::AddressMismatch);
            }

            (
                session.payment_request.chain.chain_type.clone(),
                session.payment_request.clone(),
            )
        };
        let verifier = self
            .verifier_registry
            .get_verifier(&chain_type)
            .ok_or(EngineError::ChainNotSupported(chain_type))?;
        let verification = verifier
            .verify_payment(&payment_request, user_address)
            .await
            .map_err(EngineError::VerificationFailed)?;
        if verification.is_paid {
            let mut sessions = self.payment_sessions_cache.write().unwrap();
            if let Some(session) = sessions.get_mut(payment_nonce) {
                session.verified = true;
            }
        }
        Ok(verification)
    }

    fn create_payment_request(
        &self,
        user_address: &str,
        resource_path: &str,
        custom_amount: Option<&str>,
    ) -> Result<PaymentRequest, EngineError> {
        let config = self.config_manager.get_config();
        let default_chain = self.config_manager.get_default_chain_config()?;
        let amount = custom_amount
            .map(|s| s.to_string())
            .unwrap_or_else(|| config.payments.default_amount.clone());
        let currency = match &config.service.default_currency {
            crate::config::CurrencyConfig {
                currency_type,
                address,
                decimals,
            } => match currency_type {
                crate::config::CurrencyType::Native => Currency::Native,
                crate::config::CurrencyType::Erc20 => {
                    let token_address =
                        address.clone().ok_or(EngineError::InvalidCurrencyConfig)?;
                    Currency::Token {
                        address: token_address,
                        decimals: *decimals,
                    }
                }
                _ => Currency::Native,
            },
        };
        Ok(PaymentRequest {
            amount,
            currency,
            recipient: self.config_manager.get_service_address(),
            chain: default_chain.clone(),
            description: Some(format!("Access to: {}", resource_path)),
            expires_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + config.payments.expiration_time_secs,
            ),
            nonce: Uuid::new_v4().to_string(),
        })
    }

    fn store_payment_session(&self, user_address: &str, payment_request: PaymentRequest) {
        let session = PaymentSession {
            user_address: user_address.to_string(),
            payment_request,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            verified: false,
        };

        let mut sessions = self.payment_sessions_cache.write().unwrap();
        sessions.insert(session.payment_request.nonce.clone(), session);
    }

    /// Handles an access request and returns appropriate payment verification result.
    ///
    /// # Payment Flow
    ///
    /// First Request (no payment_nonce): Returns 402 Payment Required with payment details
    /// Subsequent Request (with payment_nonce): Verifies payment and grants access if paid
    /// Payment Failed/Insufficient: Returns new payment request for retry
    ///
    /// # Params
    ///
    /// user_address - Blockchain address of the user requesting access
    /// resource_path - Path identifier for the requested resource (used in payment description)
    /// payment_nonce - Optional payment session identifier from previous 402 response
    /// custom_amount - Optional custom payment amount overriding default configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// // First request - returns 402 with payment details
    /// let result = engine.handle_access_request(
    ///     "0x1234...",
    ///     "/premium/content",
    ///     None,
    ///     None
    /// ).await?;
    ///
    /// // Subsequent request with payment nonce - returns 200 if paid
    /// let result = engine.handle_access_request(
    ///     "0x1234...",
    ///     "/premium/content",
    ///     Some("payment-nonce-from-402-response"),
    ///     None
    /// ).await?;
    ///
    /// if result.should_serve_content {
    ///     // Serve the paid content to user
    /// } else {
    ///     // Return 402 response with payment details
    ///     let payment_response = result.x402_response.unwrap();
    /// }
    /// ```
    pub async fn handle_access_request(
        &self,
        user_address: &str,
        resource_path: &str,
        payment_nonce: Option<&str>,
        custom_amount: Option<&str>,
    ) -> Result<VerificationResult, EngineError> {
        if let Some(nonce) = payment_nonce {
            if let Ok(verification) = self.verify_payment(user_address, nonce).await {
                if verification.is_paid {
                    return Ok(VerificationResult {
                        should_serve_content: true,
                        http_status: 200,
                        x402_response: None,
                        verification: Some(verification),
                    });
                }
            }
        }
        let payment_request =
            self.create_payment_request(user_address, resource_path, custom_amount)?;
        let config = self.config_manager.get_config();
        let x402_response = X402ProtocolResponse {
            status: 402,
            payment_required: payment_request.clone(),
            verification_url: Some(format!(
                "{}/{}",
                config.service.base_verification_url, payment_request.nonce
            )),
        };
        self.store_payment_session(user_address, payment_request);
        Ok(VerificationResult {
            should_serve_content: false,
            http_status: 402,
            x402_response: Some(x402_response),
            verification: None,
        })
    }

    pub fn config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }

    pub fn verifier_registry(&self) -> &VerifierRegistry {
        &self.verifier_registry
    }

    pub fn verifier_registry_mut(&mut self) -> &mut VerifierRegistry {
        &mut self.verifier_registry
    }
}

#[derive(Debug)]
pub enum EngineError {
    ConfigError(ConfigError),
    VerificationError(VerificationError),
    InvalidSession,
    AddressMismatch,
    ChainNotSupported(ChainType),
    VerificationFailed(VerificationError),
    InvalidCurrencyConfig,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigError(err) => write!(f, "Configuration error: {}", err),
            Self::VerificationError(err) => write!(f, "Verification error: {}", err),
            Self::InvalidSession => write!(f, "Payment session not found"),
            Self::AddressMismatch => write!(f, "User address mismatch"),
            Self::ChainNotSupported(chain_type) => {
                write!(f, "Chain not supported: {:?}", chain_type)
            }
            Self::VerificationFailed(err) => write!(f, "Verification failed: {}", err),
            Self::InvalidCurrencyConfig => write!(f, "Invalid currency configuration"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<ConfigError> for EngineError {
    fn from(err: ConfigError) -> Self {
        Self::ConfigError(err)
    }
}

impl From<VerificationError> for EngineError {
    fn from(err: VerificationError) -> Self {
        Self::VerificationError(err)
    }
}

struct PaymentSession {
    user_address: String,
    payment_request: PaymentRequest,
    created_at: u64,
    verified: bool,
}
