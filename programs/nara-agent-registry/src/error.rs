use anchor_lang::prelude::*;

#[error_code]
pub enum AgentRegistryError {
    #[msg("Agent ID too short: min 5 bytes")]
    AgentIdTooShort,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Buffer write offset mismatch: writes must be sequential")]
    OffsetMismatch,
    #[msg("Write out of bounds")]
    WriteOutOfBounds,
    #[msg("Buffer not fully written")]
    BufferIncomplete,
    #[msg("A pending buffer already exists; call close_buffer first")]
    PendingBufferExists,
    #[msg("Buffer account size does not match total_len")]
    InvalidBufferSize,
    #[msg("Buffer account must be owned by this program")]
    InvalidBufferOwner,
    #[msg("Buffer account does not match agent.pending_buffer")]
    BufferMismatch,
    #[msg("Memory account must be owned by this program")]
    InvalidMemoryOwner,
    #[msg("Memory account size does not match buffer total_len")]
    InvalidMemorySize,
    #[msg("old_memory account does not match agent.memory")]
    MemoryMismatch,
    #[msg("Agent already has memory; use finalize_memory_update or finalize_memory_append instead")]
    MemoryAlreadyExists,
    #[msg("Agent has no existing memory; use finalize_memory_new instead")]
    MemoryNotFound,
    #[msg("Cannot perform this operation while a pending buffer exists")]
    HasPendingBuffer,
    #[msg("Fee recipient does not match config.fee_recipient")]
    InvalidFeeRecipient,
    #[msg("Agent ID too long: max 32 bytes")]
    AgentIdTooLong,
    #[msg("No valid submit_answer instruction found in transaction")]
    QuestIxNotFound,
    #[msg("Referral agent not found")]
    ReferralNotFound,
}
