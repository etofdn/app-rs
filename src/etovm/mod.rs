//! ETOvm blockchain types and utilities.
//!
//! This module re-exports ETOvm-specific types for convenience.
//!
//! # Exports
//!
//! - Intent schemas: [`ChargeRequest`]
//! - Method details: [`EtovmMethodDetails`], [`EtovmChargeExt`]
//! - Payment modes: [`PaymentMode`]
//! - Agentic types: [`A2aTaskPayment`], [`AgentDelegateConfig`], [`SwarmBudgetAllocation`]
//! - Constants: [`CHAIN_ID`], [`METHOD_NAME`]
//!
//! # Example
//!
//! ```ignore
//! use mpp::etovm::{ChargeRequest, EtovmChargeExt, CHAIN_ID, PaymentMode};
//!
//! let req: ChargeRequest = challenge.request.decode()?;
//! match req.payment_mode() {
//!     PaymentMode::Direct => { /* standard SPL transfer */ }
//!     PaymentMode::AgentDelegate => { /* LLM-controlled spending */ }
//!     PaymentMode::A2aEscrow => { /* task-based escrow */ }
//!     PaymentMode::SwarmBudget => { /* multi-agent budget */ }
//! }
//! ```

pub use crate::protocol::intents::ChargeRequest;
pub use crate::protocol::methods::etovm::{
    charge_challenge, charge_challenge_with_options, EtovmChargeExt, EtovmCredentialPayload,
    EtovmMethodDetails, EtovmNetwork, PaymentMode, CHAIN_ID, DEFAULT_EXPIRES_MINUTES,
    DEFAULT_RPC_URL, INTENT_CHARGE, METHOD_NAME, TESTNET_CHAIN_ID, TESTNET_RPC_URL,
};

// Program IDs
pub use crate::protocol::methods::etovm::{
    A2A_PROGRAM_ID, AGENT_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, MCP_PROGRAM_ID,
    SWARM_PROGRAM_ID, TOKEN_PROGRAM_ID,
};

// Agentic payment types
pub use crate::protocol::methods::etovm::agent::{
    A2aSettlementReceipt, A2aTaskPayment, A2aTaskStatus, AgentDelegateConfig,
    AgentDelegateReceipt, McpToolPayment, SwarmBudgetAllocation, SwarmTaskReceipt,
    SwarmTaskReward,
};

#[cfg(feature = "server")]
pub use crate::protocol::methods::etovm::ChargeMethod as EtovmChargeMethod;
