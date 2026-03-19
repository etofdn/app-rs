//! ETOvm charge extension trait for `ChargeRequest`.
//!
//! Provides ETOvm-specific accessors for parsing Solana-style addresses,
//! agent parameters, and payment mode from a generic `ChargeRequest`.

use crate::protocol::intents::ChargeRequest;

use super::types::{EtovmMethodDetails, PaymentMode};

/// Extension trait adding ETOvm-specific accessors to `ChargeRequest`.
///
/// # Examples
///
/// ```ignore
/// use mpp::protocol::methods::etovm::EtovmChargeExt;
///
/// let req: ChargeRequest = challenge.request.decode().unwrap();
/// let mode = req.payment_mode();
/// let recipient = req.recipient_pubkey().unwrap();
/// ```
pub trait EtovmChargeExt {
    /// Parse the ETOvm method details from the request.
    fn etovm_details(&self) -> Option<EtovmMethodDetails>;

    /// Get the chain ID from method details, or the default (43114).
    fn chain_id(&self) -> u64;

    /// Whether the server should sponsor transaction fees.
    fn fee_payer(&self) -> bool;

    /// Get the memo if present.
    fn memo(&self) -> Option<String>;

    /// Get the agent ID for delegate transfers.
    fn agent_id(&self) -> Option<String>;

    /// Get the A2A task ID for escrow payments.
    fn a2a_task_id(&self) -> Option<String>;

    /// Get the swarm ID for budget distribution.
    fn swarm_id(&self) -> Option<String>;

    /// Get the token mint address (base58).
    fn mint(&self) -> Option<String>;

    /// Determine the payment mode based on method details.
    ///
    /// Priority: swarm > a2a > agent_delegate > direct
    fn payment_mode(&self) -> PaymentMode;
}

impl EtovmChargeExt for ChargeRequest {
    fn etovm_details(&self) -> Option<EtovmMethodDetails> {
        self.method_details
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    fn chain_id(&self) -> u64 {
        self.etovm_details()
            .and_then(|d| d.chain_id)
            .unwrap_or(super::CHAIN_ID)
    }

    fn fee_payer(&self) -> bool {
        self.etovm_details()
            .and_then(|d| d.fee_payer)
            .unwrap_or(false)
    }

    fn memo(&self) -> Option<String> {
        self.etovm_details().and_then(|d| d.memo)
    }

    fn agent_id(&self) -> Option<String> {
        self.etovm_details().and_then(|d| d.agent_id)
    }

    fn a2a_task_id(&self) -> Option<String> {
        self.etovm_details().and_then(|d| d.a2a_task_id)
    }

    fn swarm_id(&self) -> Option<String> {
        self.etovm_details().and_then(|d| d.swarm_id)
    }

    fn mint(&self) -> Option<String> {
        self.etovm_details().and_then(|d| d.mint)
    }

    fn payment_mode(&self) -> PaymentMode {
        let details = match self.etovm_details() {
            Some(d) => d,
            None => return PaymentMode::Direct,
        };

        if details.swarm_id.is_some() {
            PaymentMode::SwarmBudget
        } else if details.a2a_task_id.is_some() {
            PaymentMode::A2aEscrow
        } else if details.agent_id.is_some() {
            PaymentMode::AgentDelegate
        } else {
            PaymentMode::Direct
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(details: Option<serde_json::Value>) -> ChargeRequest {
        ChargeRequest {
            amount: "1000000".into(),
            currency: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".into(),
            recipient: Some("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".into()),
            method_details: details,
            ..Default::default()
        }
    }

    #[test]
    fn test_chain_id_default() {
        let req = make_request(None);
        assert_eq!(req.chain_id(), super::super::CHAIN_ID);
    }

    #[test]
    fn test_chain_id_from_details() {
        let req = make_request(Some(serde_json::json!({ "chainId": 99999 })));
        assert_eq!(req.chain_id(), 99999);
    }

    #[test]
    fn test_fee_payer_default_false() {
        let req = make_request(None);
        assert!(!req.fee_payer());
    }

    #[test]
    fn test_fee_payer_true() {
        let req = make_request(Some(serde_json::json!({ "feePayer": true })));
        assert!(req.fee_payer());
    }

    #[test]
    fn test_payment_mode_direct() {
        let req = make_request(None);
        assert_eq!(req.payment_mode(), PaymentMode::Direct);
    }

    #[test]
    fn test_payment_mode_agent_delegate() {
        let req = make_request(Some(serde_json::json!({ "agentId": "Agent123" })));
        assert_eq!(req.payment_mode(), PaymentMode::AgentDelegate);
    }

    #[test]
    fn test_payment_mode_a2a_escrow() {
        let req = make_request(Some(serde_json::json!({ "a2aTaskId": "Task456" })));
        assert_eq!(req.payment_mode(), PaymentMode::A2aEscrow);
    }

    #[test]
    fn test_payment_mode_swarm_budget() {
        let req = make_request(Some(serde_json::json!({ "swarmId": "Swarm789" })));
        assert_eq!(req.payment_mode(), PaymentMode::SwarmBudget);
    }

    #[test]
    fn test_payment_mode_priority_swarm_over_a2a() {
        let req = make_request(Some(serde_json::json!({
            "swarmId": "Swarm789",
            "a2aTaskId": "Task456"
        })));
        assert_eq!(req.payment_mode(), PaymentMode::SwarmBudget);
    }

    #[test]
    fn test_mint_accessor() {
        let req = make_request(Some(serde_json::json!({
            "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        })));
        assert_eq!(
            req.mint().unwrap(),
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }
}
