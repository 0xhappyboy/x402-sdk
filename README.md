<h1 align="center">
    x402 SDK
</h1>
<h4 align="center">
A Rust implementation of the x402 protocol, inspired by @coinbase/x402.
X402 is a blockchain payment protocol SDK based on the HTTP 402 Payment Required status code, providing developers with a complete on-chain payment verification solution. 
</h4>
<p align="center">
<img src="https://img.shields.io/badge/X402-Payment_Protocol-8A2BE2.svg?style=flat&labelColor=2F1E2E&color=FFB6C1&logo=ethereum&label=Protocol&logoColor=FFB6C1" alt="License">
<img src="https://img.shields.io/badge/Rust-1.70%2B-FF7F50.svg?style=flat&labelColor=2E2F1E&color=98FB98&logo=rust&label=Language&logoColor=98FB98" alt="License">
<a href="https://github.com/0xhappyboy/x402-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-Apache2.0-9370DB.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=License&logoColor=BEC5C9" alt="License"></a>
<a href="https://crates.io/crates/x402-sdk">
<img src="https://img.shields.io/badge/crates-x402--sdk-20B2AA.svg?style=flat&labelColor=0F1F2D&color=FFD700&logo=rust&logoColor=FFD700">
</a>
</p>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

# 📦 Depend

```
cargo add x402-sdk
```

# 🏗 Architecture overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ client request  │──▶│    X402 Core     │ ──▶│  Pay validator  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ HTTP 402 Status │    │  Session Cache   │    │   Block SDK     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

# 🚀Multi-chain Support

| **OrderType** | **progress** |
| :------------ | :----------- |
| **Ethereum**  | ✅           |
| **Base**      | ✅           |
| **BSC**       | ⏳           |
| **Solana**    | ✅           |
| **Aptos**     | ⏳           |
| **Sui**       | ⏳           |

# 🎮 Example

## Axum

```rust
use axum::{extract::State, http::StatusCode, response::Json, Router};
use std::sync::Arc;
use x402::core::X402;

struct AppState {
    x402_engine: Arc<X402>,
}

// http pay interface request processing
async fn access_premium_content(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(resource_path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get the user address and payment nonce from the request header specified by X402
    let user_address = headers
        .get("x-402-user-address")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let payment_nonce = headers
        .get("x-402-payment-nonce")
        .and_then(|h| h.to_str().ok());
    let result = state.x402_engine
        .handle_access_request(user_address, &resource_path, payment_nonce, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.should_serve_content {
        // Paid, return content
        Ok(Json(serde_json::json!({
            "status": "success",
            "content": format!("...", resource_path)
        })))
    } else {
        // Payment is required, return payment information
        let payment_info = result.x402_response.unwrap();
        Ok(Json(serde_json::json!({
            "status": "payment_required",
            "amount": payment_info.payment_required.amount,
            "recipient": payment_info.payment_required.recipient,
            "nonce": payment_info.payment_required.nonce
        })))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x402_engine = X402::from_default_config()?;
    // Register the specified chain
    x402_engine.register_chain_verifier(
        x402::types::ChainType::ethereum(),
        "https://eth.llamarpc.com".to_string()
    ).await?;
    let app = Router::new()
        .route("/premium", axum::routing::get(access_premium_content))
        .with_state(Arc::new(AppState {
            x402_engine: Arc::new(x402_engine)
        }));
    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await?,
        app
    ).await?;
    Ok(())
}
```
