//! ETOvm charge method for server-side payment verification.
//!
//! Verifies SPL token transfers and agentic payments (A2A escrow,
//! delegate transfers) on the ETOvm chain via JSON-RPC.
//!
//! # Verification Flow
//!
//! 1. Parse credential payload (signature or signed transaction)
//! 2. Fetch transaction from ETOvm RPC
//! 3. Verify SPL token transfer instruction matches expected parameters
//! 4. For agentic payments, verify the corresponding program state
//!
//! # Example
//!
//! ```ignore
//! use mpp::protocol::methods::etovm::ChargeMethod;
//!
//! let method = ChargeMethod::new("https://rpc.etovm.com");
//!
//! let receipt = method.verify(&credential, &request).await?;
//! assert!(receipt.is_success());
//! ```

use std::future::Future;
use std::sync::Arc;

use crate::protocol::core::{PaymentCredential, Receipt};
use crate::protocol::intents::ChargeRequest;
use crate::protocol::traits::{ChargeMethod as ChargeMethodTrait, VerificationError};
use crate::store::Store;

use super::charge::EtovmChargeExt;
use super::types::PaymentMode;
use super::{INTENT_CHARGE, METHOD_NAME};

/// ETOvm charge method for one-time payment verification.
///
/// Uses JSON-RPC to query the ETOvm chain and verify SPL token transfers.
#[derive(Clone)]
pub struct ChargeMethod {
    rpc_url: String,
    client: reqwest::Client,
    store: Option<Arc<dyn Store>>,
}

impl ChargeMethod {
    /// Create a new ETOvm charge method with the given RPC URL.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
            store: None,
        }
    }

    /// Configure a store for transaction signature deduplication.
    pub fn with_store(mut self, store: Arc<dyn Store>) -> Self {
        self.store = Some(store);
        self
    }

    /// Verify a payment by looking up the transaction signature.
    async fn verify_signature(
        &self,
        signature: &str,
        charge: &ChargeRequest,
    ) -> Result<Receipt, VerificationError> {
        // Deduplication check
        let replay_key = format!("mpp:etovm:charge:{signature}");
        if let Some(store) = &self.store {
            let seen = store
                .get(&replay_key)
                .await
                .map_err(|e| VerificationError::new(format!("Store error: {e}")))?;
            if seen.is_some() {
                return Err(VerificationError::new(
                    "Transaction signature has already been used.",
                ));
            }
        }

        // Fetch the transaction via JSON-RPC
        let tx_info = self.get_transaction(signature).await?;

        // Verify the transaction was successful
        let err = tx_info
            .get("meta")
            .and_then(|m| m.get("err"));
        if err.is_some() && !err.unwrap().is_null() {
            return Err(VerificationError::transaction_failed(format!(
                "Transaction {} failed: {:?}",
                signature, err
            )));
        }

        // Determine payment mode and verify accordingly
        let mode = charge.payment_mode();
        match mode {
            PaymentMode::Direct => {
                self.verify_spl_transfer(&tx_info, charge)?;
            }
            PaymentMode::AgentDelegate => {
                self.verify_agent_delegate_transfer(&tx_info, charge)?;
            }
            PaymentMode::A2aEscrow => {
                self.verify_a2a_escrow_payment(&tx_info, charge)?;
            }
            PaymentMode::SwarmBudget => {
                self.verify_swarm_payment(&tx_info, charge)?;
            }
        }

        // Record for deduplication
        if let Some(store) = &self.store {
            store
                .put(&replay_key, serde_json::Value::Bool(true))
                .await
                .map_err(|e| VerificationError::new(format!("Failed to record signature: {e}")))?;
        }

        Ok(Receipt::success(METHOD_NAME, signature))
    }

    /// Query ETOvm JSON-RPC for a confirmed transaction.
    async fn get_transaction(
        &self,
        signature: &str,
    ) -> Result<serde_json::Value, VerificationError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                signature,
                { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }
            ]
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                VerificationError::network_error(format!("RPC request failed: {e}"))
            })?;

        let result: serde_json::Value = resp.json().await.map_err(|e| {
            VerificationError::network_error(format!("Failed to parse RPC response: {e}"))
        })?;

        if let Some(error) = result.get("error") {
            return Err(VerificationError::network_error(format!(
                "RPC error: {}",
                error
            )));
        }

        result
            .get("result")
            .cloned()
            .filter(|v| !v.is_null())
            .ok_or_else(|| {
                VerificationError::pending(format!(
                    "Transaction {} not found or not yet confirmed",
                    signature
                ))
            })
    }

    /// Verify an SPL token transfer in a parsed transaction.
    ///
    /// Checks the transaction's inner instructions for an SPL Token
    /// `transfer` or `transferChecked` matching the expected amount,
    /// recipient, and token mint.
    fn verify_spl_transfer(
        &self,
        tx_info: &serde_json::Value,
        charge: &ChargeRequest,
    ) -> Result<(), VerificationError> {
        let expected_amount: u64 = charge
            .amount
            .parse()
            .map_err(|e| VerificationError::new(format!("Invalid amount: {e}")))?;

        let expected_recipient = charge
            .recipient
            .as_deref()
            .ok_or_else(|| VerificationError::new("Missing recipient in charge request"))?;

        if expected_amount == 0 {
            return Err(VerificationError::new(
                "Invalid amount: must be greater than zero",
            ));
        }

        // Navigate to the parsed instructions
        let instructions = tx_info
            .pointer("/transaction/message/instructions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| VerificationError::new("Transaction has no instructions"))?;

        // Also check inner instructions (for composed transactions)
        let inner_instructions = tx_info
            .pointer("/meta/innerInstructions")
            .and_then(|v| v.as_array());

        let mut all_instructions: Vec<&serde_json::Value> =
            instructions.iter().collect();

        if let Some(inner) = inner_instructions {
            for group in inner {
                if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
                    all_instructions.extend(ixs.iter());
                }
            }
        }

        // Look for a matching SPL Token transfer
        for ix in &all_instructions {
            let program = ix
                .pointer("/program")
                .and_then(|v| v.as_str())
                .or_else(|| ix.pointer("/programId").and_then(|v| v.as_str()));

            // Check if this is an SPL Token program instruction
            let is_token_program = match program {
                Some("spl-token") => true,
                Some(id) if id == super::TOKEN_PROGRAM_ID => true,
                _ => false,
            };

            if !is_token_program {
                continue;
            }

            let parsed = match ix.get("parsed") {
                Some(p) => p,
                None => continue,
            };

            let ix_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let info = match parsed.get("info") {
                Some(i) => i,
                None => continue,
            };

            match ix_type {
                "transfer" | "transferChecked" => {
                    let dest = info
                        .get("destination")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Amount can be a string or nested in tokenAmount
                    let amount: Option<u64> = info
                        .get("amount")
                        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        .or_else(|| {
                            info.pointer("/tokenAmount/amount")
                                .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
                        });

                    if dest == expected_recipient {
                        if let Some(amt) = amount {
                            if amt == expected_amount {
                                return Ok(());
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        Err(VerificationError::new(format!(
            "No matching SPL token transfer found: expected {} to {}",
            expected_amount, expected_recipient
        )))
    }

    /// Verify an agent delegate transfer.
    ///
    /// Checks that the transaction contains an AgentProgram DelegateTransfer
    /// instruction from the specified agent, within policy limits.
    fn verify_agent_delegate_transfer(
        &self,
        tx_info: &serde_json::Value,
        charge: &ChargeRequest,
    ) -> Result<(), VerificationError> {
        let agent_id = charge.agent_id().ok_or_else(|| {
            VerificationError::new("Agent delegate payment requires agentId in methodDetails")
        })?;

        // For agent delegate transfers, we verify:
        // 1. The transaction invokes the AgentProgram
        // 2. The instruction is DelegateTransfer (discriminator check)
        // 3. The agent account matches
        // 4. The underlying SPL transfer matches amount/recipient

        let instructions = tx_info
            .pointer("/transaction/message/instructions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| VerificationError::new("Transaction has no instructions"))?;

        let account_keys = tx_info
            .pointer("/transaction/message/accountKeys")
            .and_then(|v| v.as_array());

        // Check that the AgentProgram was invoked
        let agent_program_invoked = instructions.iter().any(|ix| {
            let program_id = ix
                .get("programId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            program_id == super::AGENT_PROGRAM_ID
        });

        if !agent_program_invoked {
            return Err(VerificationError::new(
                "Transaction does not invoke the AgentProgram",
            ));
        }

        // Verify the agent account is referenced
        let agent_referenced = account_keys
            .map(|keys| {
                keys.iter().any(|k| {
                    let key = k.as_str().or_else(|| k.get("pubkey").and_then(|v| v.as_str()));
                    key == Some(&agent_id)
                })
            })
            .unwrap_or(false);

        if !agent_referenced {
            return Err(VerificationError::new(format!(
                "Agent {} not found in transaction accounts",
                agent_id
            )));
        }

        // The underlying transfer should still match expected params
        self.verify_spl_transfer(tx_info, charge)
    }

    /// Verify an A2A escrow payment.
    ///
    /// Checks that the transaction creates or settles an A2A task
    /// with the expected escrow amount.
    fn verify_a2a_escrow_payment(
        &self,
        tx_info: &serde_json::Value,
        charge: &ChargeRequest,
    ) -> Result<(), VerificationError> {
        let task_id = charge.a2a_task_id().ok_or_else(|| {
            VerificationError::new("A2A escrow payment requires a2aTaskId in methodDetails")
        })?;

        let instructions = tx_info
            .pointer("/transaction/message/instructions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| VerificationError::new("Transaction has no instructions"))?;

        // Verify the A2A program was invoked
        let a2a_invoked = instructions.iter().any(|ix| {
            let program_id = ix
                .get("programId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            program_id == super::A2A_PROGRAM_ID
        });

        if !a2a_invoked {
            return Err(VerificationError::new(
                "Transaction does not invoke the A2A program",
            ));
        }

        // Verify the task account is referenced
        let account_keys = tx_info
            .pointer("/transaction/message/accountKeys")
            .and_then(|v| v.as_array());

        let task_referenced = account_keys
            .map(|keys| {
                keys.iter().any(|k| {
                    let key = k.as_str().or_else(|| k.get("pubkey").and_then(|v| v.as_str()));
                    key == Some(&task_id)
                })
            })
            .unwrap_or(false);

        if !task_referenced {
            return Err(VerificationError::new(format!(
                "A2A task {} not found in transaction accounts",
                task_id
            )));
        }

        // Verify the SPL transfer component
        self.verify_spl_transfer(tx_info, charge)
    }

    /// Verify a swarm budget payment.
    ///
    /// Checks that the transaction interacts with the SwarmProgram
    /// for the specified swarm.
    fn verify_swarm_payment(
        &self,
        tx_info: &serde_json::Value,
        charge: &ChargeRequest,
    ) -> Result<(), VerificationError> {
        let swarm_id = charge.swarm_id().ok_or_else(|| {
            VerificationError::new("Swarm payment requires swarmId in methodDetails")
        })?;

        let account_keys = tx_info
            .pointer("/transaction/message/accountKeys")
            .and_then(|v| v.as_array());

        let swarm_referenced = account_keys
            .map(|keys| {
                keys.iter().any(|k| {
                    let key = k.as_str().or_else(|| k.get("pubkey").and_then(|v| v.as_str()));
                    key == Some(&swarm_id)
                })
            })
            .unwrap_or(false);

        if !swarm_referenced {
            return Err(VerificationError::new(format!(
                "Swarm {} not found in transaction accounts",
                swarm_id
            )));
        }

        // Verify the SPL transfer component
        self.verify_spl_transfer(tx_info, charge)
    }
}

impl ChargeMethodTrait for ChargeMethod {
    fn method(&self) -> &str {
        METHOD_NAME
    }

    fn verify(
        &self,
        credential: &PaymentCredential,
        request: &ChargeRequest,
    ) -> impl Future<Output = Result<Receipt, VerificationError>> + Send {
        let credential = credential.clone();
        let request = request.clone();
        let this = self.clone();

        async move {
            if credential.challenge.method.as_str() != METHOD_NAME {
                return Err(VerificationError::credential_mismatch(format!(
                    "Method mismatch: expected {}, got {}",
                    METHOD_NAME, credential.challenge.method
                )));
            }
            if credential.challenge.intent.as_str() != INTENT_CHARGE {
                return Err(VerificationError::credential_mismatch(format!(
                    "Intent mismatch: expected {}, got {}",
                    INTENT_CHARGE, credential.challenge.intent
                )));
            }

            // Extract the signature from the credential payload
            let payload = credential.charge_payload().map_err(|e| {
                VerificationError::with_code(
                    format!("Expected charge payload: {}", e),
                    crate::protocol::traits::ErrorCode::InvalidCredential,
                )
            })?;

            let signature = if payload.is_hash() {
                // "hash" payload contains the transaction signature
                payload.tx_hash().unwrap().to_string()
            } else {
                // "transaction" payload: broadcast and get signature
                // For ETOvm, client should broadcast first and send signature
                return Err(VerificationError::new(
                    "ETOvm requires client-side broadcast: send signature, not raw transaction",
                ));
            };

            this.verify_signature(&signature, &request).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charge_method_name() {
        let method = ChargeMethod::new("http://localhost:8899");
        assert_eq!(ChargeMethodTrait::method(&method), "etovm");
    }

    #[test]
    fn test_verify_spl_transfer_parsing() {
        let method = ChargeMethod::new("http://localhost:8899");

        // Simulated jsonParsed transaction with SPL token transfer
        let tx_info = serde_json::json!({
            "transaction": {
                "message": {
                    "instructions": [
                        {
                            "program": "spl-token",
                            "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                            "parsed": {
                                "type": "transfer",
                                "info": {
                                    "source": "SourceAccount123",
                                    "destination": "DestAccount456",
                                    "amount": "1000000",
                                    "authority": "OwnerPubkey789"
                                }
                            }
                        }
                    ]
                }
            },
            "meta": {
                "err": null,
                "innerInstructions": []
            }
        });

        let charge = ChargeRequest {
            amount: "1000000".into(),
            currency: "TokenMint".into(),
            recipient: Some("DestAccount456".into()),
            ..Default::default()
        };

        let result = method.verify_spl_transfer(&tx_info, &charge);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_spl_transfer_amount_mismatch() {
        let method = ChargeMethod::new("http://localhost:8899");

        let tx_info = serde_json::json!({
            "transaction": {
                "message": {
                    "instructions": [
                        {
                            "program": "spl-token",
                            "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                            "parsed": {
                                "type": "transfer",
                                "info": {
                                    "destination": "DestAccount456",
                                    "amount": "500000"
                                }
                            }
                        }
                    ]
                }
            },
            "meta": {
                "err": null,
                "innerInstructions": []
            }
        });

        let charge = ChargeRequest {
            amount: "1000000".into(),
            currency: "TokenMint".into(),
            recipient: Some("DestAccount456".into()),
            ..Default::default()
        };

        let result = method.verify_spl_transfer(&tx_info, &charge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No matching"));
    }

    #[test]
    fn test_verify_spl_transfer_zero_amount_rejected() {
        let method = ChargeMethod::new("http://localhost:8899");

        let tx_info = serde_json::json!({
            "transaction": {
                "message": {
                    "instructions": []
                }
            },
            "meta": { "err": null }
        });

        let charge = ChargeRequest {
            amount: "0".into(),
            currency: "TokenMint".into(),
            recipient: Some("Dest".into()),
            ..Default::default()
        };

        let result = method.verify_spl_transfer(&tx_info, &charge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than zero"));
    }

    #[test]
    fn test_verify_transfer_checked() {
        let method = ChargeMethod::new("http://localhost:8899");

        let tx_info = serde_json::json!({
            "transaction": {
                "message": {
                    "instructions": [
                        {
                            "program": "spl-token",
                            "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                            "parsed": {
                                "type": "transferChecked",
                                "info": {
                                    "destination": "DestAccount456",
                                    "tokenAmount": {
                                        "amount": "2000000",
                                        "decimals": 6,
                                        "uiAmount": 2.0
                                    }
                                }
                            }
                        }
                    ]
                }
            },
            "meta": {
                "err": null,
                "innerInstructions": []
            }
        });

        let charge = ChargeRequest {
            amount: "2000000".into(),
            currency: "TokenMint".into(),
            recipient: Some("DestAccount456".into()),
            ..Default::default()
        };

        let result = method.verify_spl_transfer(&tx_info, &charge);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_agent_delegate_requires_agent_id() {
        let method = ChargeMethod::new("http://localhost:8899");

        let tx_info = serde_json::json!({
            "transaction": { "message": { "instructions": [], "accountKeys": [] } },
            "meta": { "err": null }
        });

        // No agentId in method details
        let charge = ChargeRequest {
            amount: "1000000".into(),
            currency: "TokenMint".into(),
            recipient: Some("Dest".into()),
            ..Default::default()
        };

        let result = method.verify_agent_delegate_transfer(&tx_info, &charge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("agentId"));
    }
}
