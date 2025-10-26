<h1 align="center">
    x402 SDK
</h1>
<h4 align="center">
a Rust implementation of the x402 protocol, inspired by @coinbase/x402
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/x402-sdk/LICENSE"><img src="https://img.shields.io/badge/License-Apache2.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
</p>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

# Example

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
