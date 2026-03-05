use anchor_lang::prelude::*;

/// PDA for an agent's custom metadata, seeds = [b"meta", agent_record.key()].
/// Created lazily on first `set_metadata` call. Dynamically sized —
/// account is reallocated on each update to fit the new data.
#[account]
pub struct AgentMetadata {
    /// Arbitrary data string (typically JSON), no max length
    /// (limited only by transaction size).
    pub data: String,
}

impl AgentMetadata {
    /// Calculate space needed for a given data length.
    pub fn space(data_len: usize) -> usize {
        8 + 4 + data_len // discriminator + String prefix + bytes
    }
}
