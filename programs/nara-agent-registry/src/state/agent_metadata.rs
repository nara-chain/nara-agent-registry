use anchor_lang::prelude::*;

/// PDA for an agent's metadata, seeds = [b"meta", agent_record.key()].
/// Zero-copy header followed by dynamic data content.
/// Layout: [8 disc][64 reserved][4 data_len][data_bytes...]
#[account(zero_copy)]
#[repr(C)]
pub struct AgentMetadata {
    pub _reserved: [u8; 64],
}

impl AgentMetadata {
    pub const HEADER_SIZE: usize = 8 + std::mem::size_of::<Self>();

    /// Total space needed: header + 4-byte length prefix + data bytes.
    pub fn space(data_len: usize) -> usize {
        Self::HEADER_SIZE + 4 + data_len
    }
}
