use anchor_lang::prelude::*;

/// Global PDA that tracks all pending twitter verification requests.
/// Seeds: [b"twitter_queue"]
///
/// Account layout: [8 disc][8 struct (len)][32*N Pubkeys]
///   - `len` = number of active entries
///   - Pubkeys begin at byte offset HEADER_SIZE (16)
///   - capacity = (data_len - HEADER_SIZE) / ENTRY_SIZE
///   - When len == capacity the account is resized by one slot before writing
#[account(zero_copy)]
#[repr(C)]
pub struct TwitterQueue {
    /// Number of AgentTwitter PDA addresses currently pending verification.
    pub len: u64,
}

impl TwitterQueue {
    /// Byte offset where Pubkey entries begin: discriminator(8) + struct(8).
    pub const HEADER_SIZE: usize = 8 + std::mem::size_of::<Self>();
    /// Size of each entry (one Pubkey).
    pub const ENTRY_SIZE: usize = 32;
}
