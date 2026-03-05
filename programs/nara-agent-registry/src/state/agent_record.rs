use anchor_lang::prelude::*;

/// PDA metadata account for an agent, seeds = [b"agent", agent_id.as_bytes()].
/// Stores the authority, a pointer to the memory account, and an optional
/// pending-buffer pointer. Created by the contract via `register_agent`.
#[account]
pub struct AgentRecord {
    /// Who may update this agent.
    pub authority: Pubkey,
    /// Globally unique agent ID (min 5 bytes, max 32 bytes enforced by Solana PDA seed limit).
    pub agent_id: String,
    /// Active upload buffer, if any. Must be closed before starting a new one.
    pub pending_buffer: Option<Pubkey>,
    /// Current AgentMemory account. Pubkey::default() = no memory yet.
    pub memory: Pubkey,
    /// Memory version. 0 = no memory yet, set to 1 on first upload,
    /// incremented by 1 on every subsequent update/append.
    pub version: u32,
    /// Unix timestamp when the agent was first registered.
    pub created_at: i64,
    /// Unix timestamp of the last memory update (0 = no memory yet).
    pub updated_at: i64,
}

impl AgentRecord {
    /// Byte size for `init` space calculation.
    pub fn space(agent_id_len: usize) -> usize {
        8                   // discriminator
        + 32                // authority
        + 4 + agent_id_len  // agent_id (String: u32 prefix + bytes)
        + 1 + 32            // Option<Pubkey>
        + 32                // memory
        + 4                 // version
        + 8                 // created_at
        + 8                 // updated_at
    }
}
