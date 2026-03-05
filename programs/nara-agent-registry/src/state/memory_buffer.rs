use anchor_lang::prelude::*;

/// Client-created zero-copy account (owner = program) used for chunked uploads.
/// Fixed header (80 bytes) followed by raw data bytes.
///
/// The client calls `system_program::create_account` with
///   `space = MemoryBuffer::required_size(total_len), owner = program_id`
/// then calls `init_buffer`, which uses `load_init()` to write the header.
/// Subsequent `write_to_buffer` calls advance `write_offset` sequentially.
#[account(zero_copy)]
#[repr(C)]
pub struct MemoryBuffer {
    /// Must match the AgentRecord's authority.
    pub authority: Pubkey,
    /// The AgentRecord PDA this buffer is uploading to.
    pub agent: Pubkey,
    /// Expected total number of data bytes.
    pub total_len: u32,
    /// Current write cursor. Each `write_to_buffer` call advances this.
    /// Client supplies the expected offset; contract rejects mismatches.
    pub write_offset: u32,
    // Raw data bytes follow at offset HEADER_SIZE (not declared as a field).
}

impl MemoryBuffer {
    /// Discriminator (8) + authority (32) + agent (32) + total_len (4) + write_offset (4).
    pub const HEADER_SIZE: usize = 8 + 32 + 32 + 4 + 4;

    pub fn required_size(data_len: usize) -> usize {
        Self::HEADER_SIZE + data_len
    }
}
