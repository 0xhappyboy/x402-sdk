<h1 align="center">
       x402 SDK
</h1>
<h4 align="center">
x402 协议的 Rust 实现，灵感来自 @coinbase/x402.
X402 是基于 HTTP 402 Payment Required 状态码的区块链支付协议 SDK, 为开发者提供完整的链上支付验证解决方案。
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/x402-sdk/LICENSE"><img src="https://img.shields.io/badge/License-Apache2.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
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
│   客户端请求     │───▶│   X402 核心引擎  │───▶│  支付验证器     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  HTTP 402响应   │    │  会话管理缓存      │    │  区块链RPC      │
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

// 付费内容访问端点 http支付接口请求处理
async fn access_premium_content(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(resource_path): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 从 X402 规定的请求头获取用户地址和支付 nonce
    let user_address = headers
        .get("x-402-user-address")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let payment_nonce = headers
        .get("x-402-payment-nonce")
        .and_then(|h| h.to_str().ok());
    // 使用 X402 SDK 处理支付验证
    let result = state.x402_engine
        .handle_access_request(user_address, &resource_path, payment_nonce, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.should_serve_content {
        // 已支付，返回内容
        Ok(Json(serde_json::json!({
            "status": "success",
            "content": format!("这里是 {} 的付费内容...", resource_path)
        })))
    } else {
        // 需要支付，返回支付信息
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
    // 第三方开发者初始化 X402
    let x402_engine = X402::from_default_config()?;
    // 注册需要的区块链
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
