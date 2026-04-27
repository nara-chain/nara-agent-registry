use anchor_lang::prelude::*;

/// Reverse-lookup record: given an agent, list all index strings registered for it.
/// Seeds: [SEED_AGENT_ALIAS, agent_pda.as_ref(), index_str.as_bytes()]
///
/// Clients can fetch all aliases for a given agent via
/// `getProgramAccounts` with a memcmp filter on the `agent` field.
#[account(zero_copy)]
#[repr(C)]
pub struct AgentAlias {
    /// The AgentState PDA that owns this alias
    pub agent: Pubkey,
    /// Unix timestamp when this alias was created
    pub created_at: i64,
    /// Length of the index string stored below
    pub index_len: u32,
    pub _padding: u32,
    /// Index string (zero-padded)
    pub index: [u8; 32],
    pub _reserved: [u8; 32],
}
