use anchor_lang::prelude::*;

/// Client-created account (owner = program) that stores an agent's memory.
/// Fixed header (40 bytes) followed immediately by raw memory bytes.
///
/// For new uploads, the client calls `system_program::create_account` with
///   `space = AgentMemory::required_size(content_len), owner = program_id`
/// then passes the account to `finalize_memory_new` / `finalize_memory_update`.
///
/// For appends, `finalize_memory_append` reallocates this account in-place
/// to hold the additional data, without creating a new account.
#[account]
pub struct AgentMemory {
    /// The AgentRecord PDA this memory belongs to.
    pub agent: Pubkey,
    // Raw memory bytes follow at offset HEADER_SIZE (not declared as a Vec).
}

impl AgentMemory {
    /// Discriminator (8) + agent (32).
    pub const HEADER_SIZE: usize = 8 + 32;

    pub fn required_size(content_len: usize) -> usize {
        Self::HEADER_SIZE + content_len
    }
}
