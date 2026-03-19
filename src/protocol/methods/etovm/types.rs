//! ETOvm-specific types for Web Payment Auth.
//!
//! These types map to ETOvm's Solana-compatible account model and
//! native agent programs (A2A, MCP, Agent, Swarm).

use serde::{Deserialize, Serialize};

/// ETOvm-specific method details included in payment challenges.
///
/// These extend the base `ChargeRequest` with ETOvm chain parameters
/// and agentic payment features.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EtovmMethodDetails {
    /// Chain ID (default: 43114).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,

    /// Whether the server sponsors transaction fees (similar to Tempo fee payer).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_payer: Option<bool>,

    /// Optional memo attached to the transfer (base58-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,

    /// Agent program ID of the paying agent (for delegate transfers).
    /// When set, the payment is made via the AgentProgram's DelegateTransfer
    /// instruction, allowing LLM-controlled spending within policy limits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,

    /// A2A task ID for escrow-backed payments.
    /// When set, the payment is tied to an A2A task lifecycle:
    /// task creation → escrow lock → completion → release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a2a_task_id: Option<String>,

    /// Swarm ID for multi-agent budget distribution.
    /// Payments associated with a swarm are tracked against the
    /// swarm's pooled escrow and per-task reward allocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swarm_id: Option<String>,

    /// Token mint address (base58-encoded Pubkey).
    /// Identifies which SPL token to use for payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<String>,
}

/// Payment mode for ETOvm transactions.
///
/// ETOvm supports multiple payment modes reflecting its native
/// agent infrastructure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMode {
    /// Standard SPL token transfer between accounts.
    Direct,
    /// Agent delegate transfer (LLM-controlled, policy-bounded).
    AgentDelegate,
    /// A2A escrow-backed task payment.
    A2aEscrow,
    /// Swarm pooled budget allocation.
    SwarmBudget,
}

impl Default for PaymentMode {
    fn default() -> Self {
        Self::Direct
    }
}

/// ETOvm network configuration.
#[derive(Clone, Debug)]
pub struct EtovmNetwork {
    /// JSON-RPC endpoint URL.
    pub rpc_url: String,
    /// Chain ID.
    pub chain_id: u64,
}

impl EtovmNetwork {
    /// Create a mainnet configuration.
    pub fn mainnet() -> Self {
        Self {
            rpc_url: super::DEFAULT_RPC_URL.to_string(),
            chain_id: super::CHAIN_ID,
        }
    }

    /// Create a testnet configuration.
    pub fn testnet() -> Self {
        Self {
            rpc_url: super::TESTNET_RPC_URL.to_string(),
            chain_id: super::TESTNET_CHAIN_ID,
        }
    }
}

/// Credential payload types for ETOvm payments.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EtovmCredentialPayload {
    /// Client already broadcast the transaction; provides the signature (base58).
    Signature {
        signature: String,
    },
    /// Client provides a signed transaction for server to broadcast.
    Transaction {
        /// Base64-encoded signed transaction bytes.
        transaction: String,
    },
    /// A2A task completion proof.
    A2aCompletion {
        /// The A2A task account address (base58).
        task_id: String,
        /// Transaction signature that completed the task.
        signature: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_details_serialization() {
        let details = EtovmMethodDetails {
            chain_id: Some(43114),
            fee_payer: Some(true),
            agent_id: Some("AgentXYZ123".into()),
            ..Default::default()
        };

        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(json["chainId"], 43114);
        assert_eq!(json["feePayer"], true);
        assert_eq!(json["agentId"], "AgentXYZ123");
        // Fields that are None should be omitted
        assert!(json.get("memo").is_none());
        assert!(json.get("a2aTaskId").is_none());
    }

    #[test]
    fn test_method_details_deserialization() {
        let json = serde_json::json!({
            "chainId": 43114,
            "feePayer": true,
            "swarmId": "SwarmABC"
        });

        let details: EtovmMethodDetails = serde_json::from_value(json).unwrap();
        assert_eq!(details.chain_id, Some(43114));
        assert_eq!(details.fee_payer, Some(true));
        assert_eq!(details.swarm_id, Some("SwarmABC".into()));
        assert!(details.agent_id.is_none());
    }

    #[test]
    fn test_payment_mode_default() {
        assert_eq!(PaymentMode::default(), PaymentMode::Direct);
    }

    #[test]
    fn test_network_configs() {
        let mainnet = EtovmNetwork::mainnet();
        assert_eq!(mainnet.chain_id, 43114);

        let testnet = EtovmNetwork::testnet();
        assert_eq!(testnet.chain_id, 43115);
    }

    #[test]
    fn test_credential_payload_signature() {
        let payload = EtovmCredentialPayload::Signature {
            signature: "5VERv8NMhKEq...".into(),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["type"], "signature");
    }

    #[test]
    fn test_credential_payload_a2a() {
        let payload = EtovmCredentialPayload::A2aCompletion {
            task_id: "TaskABC".into(),
            signature: "5VERv8...".into(),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["type"], "a2aCompletion");
    }
}
