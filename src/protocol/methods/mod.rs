//! Payment method implementations for Web Payment Auth.
//!
//! This module provides method-specific types and helpers.
//!
//! # Available Methods
//!
//! - [`tempo`]: Tempo blockchain (requires `tempo` feature)
//! - [`etovm`]: ETOvm agentic chain (requires `etovm` feature)
//!
//! # Architecture
//!
//! ```text
//! methods/
//! ├── tempo/      # Tempo-specific (chain_id=42431, TIP-20, 2D nonces)
//! │   ├── types.rs    # TempoMethodDetails
//! │   └── charge.rs   # TempoChargeExt trait
//! └── etovm/      # ETOvm-specific (chain_id=43114, SPL Token, agent programs)
//!     ├── types.rs    # EtovmMethodDetails, PaymentMode
//!     ├── charge.rs   # EtovmChargeExt trait
//!     ├── agent.rs    # A2A, Agent, Swarm, MCP payment types
//!     └── method.rs   # Server-side ChargeMethod (JSON-RPC verification)
//! ```
//!
//! Shared EVM utilities (Address, U256, parsing) are in the top-level `evm` module.

#[cfg(feature = "tempo")]
pub mod tempo;

#[cfg(feature = "etovm")]
pub mod etovm;
