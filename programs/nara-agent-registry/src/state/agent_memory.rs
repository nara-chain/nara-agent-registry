use anchor_lang::prelude::*;

/// Client-created account that stores an agent's memory.
/// Zero-copy header followed by raw memory bytes.
/// Layout: [8 disc][32 agent][64 reserved][memory_bytes...]
#[account(zero_copy)]
#[repr(C)]
pub struct AgentMemory {
    pub agent: Pubkey,
    pub _reserved: [u8; 64],
}

impl AgentMemory {
    pub const DISC_SIZE: usize = 8;
    pub const AGENT_OFFSET: usize = Self::DISC_SIZE;
    pub const AGENT_END: usize = Self::AGENT_OFFSET + std::mem::size_of::<Pubkey>();
    pub const HEADER_SIZE: usize = Self::DISC_SIZE + std::mem::size_of::<Self>();

    pub fn required_size(content_len: usize) -> usize {
        Self::HEADER_SIZE + content_len
    }
}
