use anchor_lang::prelude::*;

/// PDA for an agent's bio, seeds = [b"bio", agent_record.key()].
/// Zero-copy header followed by dynamic bio content.
/// Layout: [8 disc][64 reserved][4 bio_len][bio_bytes...]
#[account(zero_copy)]
#[repr(C)]
pub struct AgentBio {
    pub _reserved: [u8; 64],
}

impl AgentBio {
    pub const HEADER_SIZE: usize = 8 + std::mem::size_of::<Self>();

    /// Total space needed: header + 4-byte length prefix + bio bytes.
    pub fn space(bio_len: usize) -> usize {
        Self::HEADER_SIZE + 4 + bio_len
    }
}
