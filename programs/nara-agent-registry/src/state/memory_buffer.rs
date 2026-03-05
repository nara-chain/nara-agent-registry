use anchor_lang::prelude::*;

/// Client-created zero-copy account for chunked uploads.
/// Fixed header followed by raw data bytes.
#[account(zero_copy)]
#[repr(C)]
pub struct MemoryBuffer {
    pub authority: Pubkey,
    pub agent: Pubkey,
    pub total_len: u32,
    pub write_offset: u32,
    pub _reserved: [u8; 64],
}

impl MemoryBuffer {
    pub const HEADER_SIZE: usize = 8 + std::mem::size_of::<Self>();

    pub fn required_size(data_len: usize) -> usize {
        Self::HEADER_SIZE + data_len
    }
}
