use anchor_lang::prelude::*;

/// Reverse-lookup record: given an agent, list all index strings registered for it.
/// Seeds: [SEED_REVERSE_INDEX, agent_pda.as_ref(), index_str.as_bytes()]
///
/// Clients can fetch all reverse_index entries for a given agent via
/// `getProgramAccounts` with a memcmp filter on the `agent` field.
#[account(zero_copy)]
#[repr(C)]
pub struct ReverseIndex {
    /// The AgentState PDA that owns this reverse-index entry
    pub agent: Pubkey,
    /// Unix timestamp when this entry was created
    pub created_at: i64,
    /// Length of the index string stored below
    pub index_len: u32,
    pub _padding: u32,
    /// Index string (zero-padded), up to 128 bytes
    pub index: [u8; 128],
    pub _reserved: [u8; 32],
}
