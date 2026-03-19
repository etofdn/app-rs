//! Agentic payment types for ETOvm's native agent infrastructure.
//!
//! ETOvm provides three on-chain programs that enable agent-native payments:
//!
//! 1. **AgentProgram** — Autonomous AI agents with delegate spending,
//!    reputation tracking, and policy-bounded transfers.
//! 2. **A2A Program** — Agent-to-agent task delegation with escrow-backed
//!    compensation and artifact exchange.
//! 3. **SwarmProgram** — DAG-based multi-agent task orchestration with
//!    pooled escrow and per-task reward distribution.
//!
//! These types integrate with the MPP charge flow to support agentic
//! payment patterns beyond simple token transfers.

use serde::{Deserialize, Serialize};

// ==================== Agent Delegate Payments ====================

/// Configuration for agent delegate transfers.
///
/// When an AI agent (LLM) needs to make payments autonomously,
/// the AgentProgram's `DelegateTransfer` instruction allows spending
/// within policy limits without requiring the authority's signature.
///
/// # Policy Constraints
///
/// The on-chain AgentProgram enforces:
/// - Per-transaction transfer limits
/// - Rolling 24-hour daily caps
/// - Recipient whitelisting
/// - Cooldown periods between transfers
/// - "Human above" thresholds requiring authority co-signature
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDelegateConfig {
    /// The agent's on-chain account address (base58).
    pub agent_address: String,

    /// The delegate keypair address that signs on behalf of the agent.
    /// This is typically the LLM's ephemeral signing key.
    pub delegate_address: String,

    /// Maximum amount per transaction (in token base units).
    pub per_tx_limit: u64,

    /// Rolling 24-hour spending cap (in token base units).
    pub daily_limit: u64,
}

/// Result of an agent delegate transfer verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDelegateReceipt {
    /// Transaction signature (base58).
    pub signature: String,

    /// The agent's on-chain address.
    pub agent_address: String,

    /// Amount transferred (in base units).
    pub amount: u64,

    /// Remaining daily budget after this transfer.
    pub remaining_daily_budget: u64,
}

// ==================== A2A Escrow Payments ====================

/// A2A (Agent-to-Agent) task payment configuration.
///
/// When one agent delegates a task to another, the A2A program
/// locks payment in escrow. Funds are released upon task completion
/// or refunded on failure/cancellation.
///
/// # Task Lifecycle
///
/// ```text
/// CreateTask (escrow locked)
///   → AcceptTask (agent begins work)
///     → Working (messages + artifacts exchanged)
///       → CompleteTask (escrow released to worker)
///       → FailTask (escrow refunded to creator)
///       → CancelTask (escrow refunded to creator)
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aTaskPayment {
    /// The A2A task account address (base58).
    pub task_address: String,

    /// The requesting agent's address (task creator / payer).
    pub requester: String,

    /// The fulfilling agent's address (task worker / payee).
    pub fulfiller: String,

    /// Escrow amount locked for this task (in token base units).
    pub escrow_amount: u64,

    /// Token mint for the escrow.
    pub mint: String,

    /// Current task status.
    pub status: A2aTaskStatus,
}

/// Status of an A2A task for payment tracking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum A2aTaskStatus {
    /// Task submitted, escrow locked.
    Submitted,
    /// Agent accepted and is working.
    Working,
    /// Task completed, escrow released to fulfiller.
    Completed,
    /// Task failed, escrow refunded to requester.
    Failed,
    /// Task cancelled, escrow refunded to requester.
    Cancelled,
}

/// Receipt for an A2A escrow payment settlement.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aSettlementReceipt {
    /// Transaction signature for the settlement (base58).
    pub signature: String,

    /// The A2A task address.
    pub task_address: String,

    /// Final status (completed or failed/cancelled).
    pub final_status: A2aTaskStatus,

    /// Amount settled (released or refunded).
    pub settled_amount: u64,

    /// Recipient of the settled funds.
    pub recipient: String,
}

// ==================== Swarm Budget Payments ====================

/// Swarm budget allocation for multi-agent task DAGs.
///
/// The SwarmProgram manages a pooled escrow that distributes
/// rewards to agents as they complete tasks in a dependency DAG.
///
/// # Budget Flow
///
/// ```text
/// CreateSwarm (total budget locked in escrow)
///   → ActivateSwarm (DAG validated)
///     → CompleteTask (agent receives reward_bps share)
///     → FinalizeSwarm (remaining budget returned to coordinator)
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBudgetAllocation {
    /// The swarm state account address (base58).
    pub swarm_address: String,

    /// The coordinator agent's address.
    pub coordinator: String,

    /// Total budget locked in the swarm escrow.
    pub total_budget: u64,

    /// Token mint for the budget.
    pub mint: String,

    /// Number of tasks in the DAG.
    pub task_count: u32,
}

/// Individual task reward within a swarm.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmTaskReward {
    /// Task index within the swarm DAG.
    pub task_index: u32,

    /// Task label.
    pub label: String,

    /// Assigned agent address (if assigned).
    pub assigned_agent: Option<String>,

    /// Reward in basis points (out of 10000).
    pub reward_bps: u32,

    /// Computed reward amount based on total budget.
    pub reward_amount: u64,
}

/// Receipt for a swarm task completion payment.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmTaskReceipt {
    /// Transaction signature (base58).
    pub signature: String,

    /// Swarm address.
    pub swarm_address: String,

    /// Completed task index.
    pub task_index: u32,

    /// Agent that completed the task.
    pub agent: String,

    /// Reward amount paid.
    pub reward_amount: u64,
}

// ==================== MCP Tool Invocation Payments ====================

/// Payment for MCP tool invocations on ETOvm.
///
/// ETOvm's native MCP program registers tools on-chain with access
/// controls. When a tool requires payment, this integrates with the
/// MPP flow: the tool's server returns a 402 challenge, the agent
/// pays, and the invocation is logged on-chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolPayment {
    /// The MCP tool account address (base58).
    pub tool_address: String,

    /// Tool name as registered on-chain.
    pub tool_name: String,

    /// The invoking agent's address.
    pub caller: String,

    /// Payment amount per invocation.
    pub amount: u64,

    /// Token mint for payment.
    pub mint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_delegate_config_serialization() {
        let config = AgentDelegateConfig {
            agent_address: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".into(),
            delegate_address: "HN7cABqLq46Es1jh92dQQisAq662SmxELLLsHHe4YWrH".into(),
            per_tx_limit: 1_000_000,
            daily_limit: 10_000_000,
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["perTxLimit"], 1_000_000);
        assert_eq!(json["dailyLimit"], 10_000_000);
    }

    #[test]
    fn test_a2a_task_status_roundtrip() {
        for status in [
            A2aTaskStatus::Submitted,
            A2aTaskStatus::Working,
            A2aTaskStatus::Completed,
            A2aTaskStatus::Failed,
            A2aTaskStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: A2aTaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_a2a_task_payment_serialization() {
        let payment = A2aTaskPayment {
            task_address: "TaskAddr123".into(),
            requester: "Agent1".into(),
            fulfiller: "Agent2".into(),
            escrow_amount: 5_000_000,
            mint: "TokenMint".into(),
            status: A2aTaskStatus::Working,
        };

        let json = serde_json::to_value(&payment).unwrap();
        assert_eq!(json["escrowAmount"], 5_000_000);
        assert_eq!(json["status"], "working");
    }

    #[test]
    fn test_swarm_budget_allocation() {
        let alloc = SwarmBudgetAllocation {
            swarm_address: "Swarm123".into(),
            coordinator: "Coordinator456".into(),
            total_budget: 100_000_000,
            mint: "USDC_Mint".into(),
            task_count: 6,
        };

        let json = serde_json::to_value(&alloc).unwrap();
        assert_eq!(json["totalBudget"], 100_000_000);
        assert_eq!(json["taskCount"], 6);
    }

    #[test]
    fn test_swarm_task_reward_bps() {
        let reward = SwarmTaskReward {
            task_index: 0,
            label: "Source gathering".into(),
            assigned_agent: Some("Agent1".into()),
            reward_bps: 500, // 5%
            reward_amount: 5_000_000,
        };

        // 500 bps = 5% of 100M = 5M
        assert_eq!(reward.reward_bps, 500);
        assert_eq!(reward.reward_amount, 5_000_000);
    }

    #[test]
    fn test_mcp_tool_payment() {
        let payment = McpToolPayment {
            tool_address: "ToolAddr789".into(),
            tool_name: "code_analysis".into(),
            caller: "AgentCaller".into(),
            amount: 100_000,
            mint: "USDC_Mint".into(),
        };

        let json = serde_json::to_value(&payment).unwrap();
        assert_eq!(json["toolName"], "code_analysis");
        assert_eq!(json["amount"], 100_000);
    }
}
