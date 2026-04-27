use anchor_lang::prelude::*;

/// Per-agent custom index entry. Lets an agent claim arbitrary index strings
/// that point back to its agent_id.
/// Seeds: [SEED_AGENT_INDEX, index_str.as_bytes()]
#[account(zero_copy)]
#[repr(C)]
pub struct AgentIndex {
    /// The AgentState PDA that owns this index entry
    pub agent: Pubkey,
    /// Unix timestamp when this index was registered
    pub created_at: i64,
    /// Length of the agent_id stored below
    pub agent_id_len: u32,
    pub _padding: u32,
    /// agent_id of the owning agent (zero-padded)
    pub agent_id: [u8; 32],
    pub _reserved: [u8; 32],
}
