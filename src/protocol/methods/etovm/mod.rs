//! ETOvm-specific types and helpers for Web Payment Auth.
//!
//! This module provides ETOvm blockchain-specific implementations for the
//! Machine Payments Protocol. ETOvm is a high-performance Universal VM with
//! Solana compatibility and native agent infrastructure (A2A, MCP, Agent, Swarm).
//!
//! # Architecture
//!
//! ETOvm uses a Solana-compatible account model with:
//! - **SPL Token program** for fungible token transfers
//! - **AgentProgram** for autonomous AI agent spending with policies
//! - **A2A program** for agent-to-agent task escrow
//! - **SwarmProgram** for multi-agent DAG orchestration with pooled budgets
//! - **MCP program** for on-chain tool registration and invocation tracking
//!
//! # Payment Modes
//!
//! | Mode | Description | Method Detail |
//! |------|-------------|---------------|
//! | Direct | Standard SPL token transfer | (default) |
//! | AgentDelegate | LLM-controlled spending within policy | `agentId` |
//! | A2aEscrow | Task-based escrow payment | `a2aTaskId` |
//! | SwarmBudget | Multi-agent budget distribution | `swarmId` |
//!
//! # Constants
//!
//! - [`CHAIN_ID`]: ETOvm mainnet chain ID (43114)
//! - [`METHOD_NAME`]: Payment method name ("etovm")
//! - [`TOKEN_PROGRAM_ID`]: SPL Token program address
//!
//! # Examples
//!
//! ## Creating a charge challenge
//!
//! ```
//! use mpp::protocol::methods::etovm;
//!
//! let challenge = etovm::charge_challenge(
//!     "my-server-secret",
//!     "api.example.com",
//!     "1000000",
//!     "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
//!     "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
//! ).unwrap();
//! assert_eq!(challenge.method.as_str(), "etovm");
//! ```
//!
//! ## Agentic payment with A2A escrow
//!
//! ```
//! use mpp::protocol::intents::ChargeRequest;
//! use mpp::protocol::methods::etovm;
//!
//! let request = ChargeRequest {
//!     amount: "5000000".into(),
//!     currency: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
//!     recipient: Some("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".into()),
//!     method_details: Some(serde_json::json!({
//!         "a2aTaskId": "TaskABC123",
//!         "chainId": 43114
//!     })),
//!     ..Default::default()
//! };
//!
//! let challenge = etovm::charge_challenge_with_options(
//!     "my-server-secret",
//!     "api.example.com",
//!     &request,
//!     None,
//!     Some("A2A audit task escrow"),
//! ).unwrap();
//! ```

pub mod agent;
pub mod charge;
pub mod types;

#[cfg(feature = "server")]
pub mod method;

pub use charge::EtovmChargeExt;
pub use types::{
    EtovmCredentialPayload, EtovmMethodDetails, EtovmNetwork, PaymentMode,
};

pub use agent::{
    A2aSettlementReceipt, A2aTaskPayment, A2aTaskStatus, AgentDelegateConfig,
    AgentDelegateReceipt, McpToolPayment, SwarmBudgetAllocation, SwarmTaskReceipt,
    SwarmTaskReward,
};

#[cfg(feature = "server")]
pub use method::ChargeMethod;

// ==================== Constants ====================

/// ETOvm mainnet chain ID.
pub const CHAIN_ID: u64 = 43114;

/// ETOvm testnet chain ID.
pub const TESTNET_CHAIN_ID: u64 = 43115;

/// Default RPC URL for ETOvm mainnet.
pub const DEFAULT_RPC_URL: &str = "https://rpc.etovm.com";

/// Default RPC URL for ETOvm testnet.
pub const TESTNET_RPC_URL: &str = "https://rpc.testnet.etovm.com";

/// Payment method name for ETOvm.
pub const METHOD_NAME: &str = "etovm";

/// Charge intent name.
pub const INTENT_CHARGE: &str = "charge";

/// Session intent name.
pub const INTENT_SESSION: &str = "session";

/// Default challenge expiration in minutes.
pub const DEFAULT_EXPIRES_MINUTES: u64 = 5;

// ==================== Program IDs ====================

/// SPL Token Program ID (base58: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA).
pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Associated Token Account Program ID.
pub const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// ETOvm Agent Program ID (placeholder — to be assigned at genesis).
pub const AGENT_PROGRAM_ID: &str = "AgentProgram1111111111111111111111111111111";

/// ETOvm A2A Program ID (placeholder — to be assigned at genesis).
pub const A2A_PROGRAM_ID: &str = "A2AProgram11111111111111111111111111111111";

/// ETOvm MCP Program ID (placeholder — to be assigned at genesis).
pub const MCP_PROGRAM_ID: &str = "MCPProgram11111111111111111111111111111111";

/// ETOvm Swarm Program ID (placeholder — to be assigned at genesis).
pub const SWARM_PROGRAM_ID: &str = "SwarmProgram1111111111111111111111111111111";

// ==================== Challenge Helpers ====================

/// Create an ETOvm charge challenge with minimal parameters.
///
/// # Arguments
///
/// * `secret_key` - Server secret key for HMAC-bound challenge ID
/// * `realm` - Protection space (e.g., "api.example.com")
/// * `amount` - Amount in token base units (e.g., "1000000" for 1 USDC)
/// * `currency` - Token mint address (base58)
/// * `recipient` - Recipient token account address (base58)
#[must_use = "returns a new PaymentChallenge"]
pub fn charge_challenge(
    secret_key: &str,
    realm: &str,
    amount: &str,
    currency: &str,
    recipient: &str,
) -> crate::error::Result<crate::protocol::core::PaymentChallenge> {
    let request = crate::protocol::intents::ChargeRequest {
        amount: amount.to_string(),
        currency: currency.to_string(),
        recipient: Some(recipient.to_string()),
        ..Default::default()
    };

    charge_challenge_with_options(secret_key, realm, &request, None, None)
}

/// Create an ETOvm charge challenge with full options.
///
/// Use this for agentic payments (A2A escrow, delegate transfers, swarm budgets).
pub fn charge_challenge_with_options(
    secret_key: &str,
    realm: &str,
    request: &crate::protocol::intents::ChargeRequest,
    expires: Option<&str>,
    description: Option<&str>,
) -> crate::error::Result<crate::protocol::core::PaymentChallenge> {
    use crate::protocol::core::{Base64UrlJson, PaymentChallenge};
    use time::{Duration, OffsetDateTime};

    let request = request.clone().with_base_units()?;
    let encoded_request = Base64UrlJson::from_typed(&request)?;

    let default_expires;
    let expires = match expires {
        Some(e) => Some(e),
        None => {
            let expiry_time =
                OffsetDateTime::now_utc() + Duration::minutes(DEFAULT_EXPIRES_MINUTES as i64);
            default_expires = expiry_time
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|e| {
                    crate::error::MppError::InvalidConfig(format!("failed to format expires: {e}"))
                })?;
            Some(default_expires.as_str())
        }
    };

    let id = crate::protocol::core::compute_challenge_id(
        secret_key,
        realm,
        METHOD_NAME,
        INTENT_CHARGE,
        encoded_request.raw(),
        expires,
        None,
        None,
    );

    Ok(PaymentChallenge {
        id,
        realm: realm.to_string(),
        method: METHOD_NAME.into(),
        intent: INTENT_CHARGE.into(),
        request: encoded_request,
        expires: expires.map(|s| s.to_string()),
        description: description.map(|s| s.to_string()),
        digest: None,
        opaque: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key";

    #[test]
    fn test_constants() {
        assert_eq!(CHAIN_ID, 43114);
        assert_eq!(TESTNET_CHAIN_ID, 43115);
        assert_eq!(METHOD_NAME, "etovm");
    }

    #[test]
    fn test_charge_challenge_basic() {
        let challenge = charge_challenge(
            TEST_SECRET,
            "api.example.com",
            "1000000",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        )
        .unwrap();

        assert_eq!(challenge.method.as_str(), METHOD_NAME);
        assert_eq!(challenge.intent.as_str(), INTENT_CHARGE);
        assert!(challenge.expires.is_some());
        assert_eq!(challenge.realm, "api.example.com");
    }

    #[test]
    fn test_charge_challenge_deterministic() {
        use crate::protocol::intents::ChargeRequest;

        let request = ChargeRequest {
            amount: "1000000".into(),
            currency: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            recipient: Some("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".into()),
            ..Default::default()
        };

        let c1 = charge_challenge_with_options(
            TEST_SECRET,
            "api.example.com",
            &request,
            Some("2026-01-01T00:00:00Z"),
            None,
        )
        .unwrap();

        let c2 = charge_challenge_with_options(
            TEST_SECRET,
            "api.example.com",
            &request,
            Some("2026-01-01T00:00:00Z"),
            None,
        )
        .unwrap();

        assert_eq!(c1.id, c2.id, "Same params should produce same ID");
    }

    #[test]
    fn test_charge_challenge_with_a2a_escrow() {
        use crate::protocol::intents::ChargeRequest;

        let request = ChargeRequest {
            amount: "5000000".into(),
            currency: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            recipient: Some("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".into()),
            method_details: Some(serde_json::json!({
                "a2aTaskId": "TaskABC123",
                "chainId": 43114
            })),
            ..Default::default()
        };

        let challenge = charge_challenge_with_options(
            TEST_SECRET,
            "api.example.com",
            &request,
            Some("2026-06-01T00:00:00Z"),
            Some("A2A audit task escrow"),
        )
        .unwrap();

        assert_eq!(challenge.method.as_str(), METHOD_NAME);
        assert_eq!(challenge.description, Some("A2A audit task escrow".into()));

        // Decode the request back to verify method details preserved
        let decoded: ChargeRequest = challenge.request.decode().unwrap();
        assert_eq!(decoded.amount, "5000000");
        let details = decoded.method_details.unwrap();
        assert_eq!(details["a2aTaskId"], "TaskABC123");
    }

    #[test]
    fn test_charge_challenge_with_swarm() {
        use crate::protocol::intents::ChargeRequest;

        let request = ChargeRequest {
            amount: "100000000".into(),
            currency: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            recipient: Some("SwarmEscrowAccount123".into()),
            method_details: Some(serde_json::json!({
                "swarmId": "Swarm789",
                "chainId": 43114
            })),
            ..Default::default()
        };

        let challenge = charge_challenge_with_options(
            TEST_SECRET,
            "api.example.com",
            &request,
            None,
            Some("Smart contract audit swarm budget"),
        )
        .unwrap();

        assert_eq!(challenge.method.as_str(), METHOD_NAME);

        let decoded: ChargeRequest = challenge.request.decode().unwrap();
        let details = decoded.method_details.unwrap();
        assert_eq!(details["swarmId"], "Swarm789");
    }

    #[test]
    fn test_challenge_id_format() {
        let challenge = charge_challenge(
            TEST_SECRET,
            "api.example.com",
            "1000000",
            "TokenMint",
            "Recipient",
        )
        .unwrap();

        // Base64url-encoded SHA256 = 43 characters
        assert_eq!(challenge.id.len(), 43);
        assert!(challenge
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_different_params_different_ids() {
        let c1 = charge_challenge(
            TEST_SECRET,
            "api.example.com",
            "1000000",
            "TokenMint",
            "Recipient",
        )
        .unwrap();

        let c2 = charge_challenge(
            TEST_SECRET,
            "api.example.com",
            "2000000", // different amount
            "TokenMint",
            "Recipient",
        )
        .unwrap();

        assert_ne!(c1.id, c2.id);
    }
}
