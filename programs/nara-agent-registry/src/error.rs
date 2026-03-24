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
    #[msg("Fee vault has insufficient balance for withdrawal")]
    InsufficientFeeVaultBalance,
    #[msg("Agent ID too long: max 32 bytes")]
    AgentIdTooLong,
    #[msg("Agent ID must be lowercase")]
    AgentIdNotLowercase,
    #[msg("No valid submit_answer instruction found in transaction")]
    QuestIxNotFound,
    #[msg("Referral agent not found")]
    ReferralNotFound,
    #[msg("Referral authority does not match referral agent's authority")]
    InvalidReferralAuthority,
    #[msg("Memory account is already initialized")]
    MemoryAlreadyInitialized,
    #[msg("referral_fee_share must not exceed referral_register_fee")]
    InvalidReferralFeeConfig,
    #[msg("referral_point_account is not the correct ATA")]
    InvalidReferralPointAccount,
    #[msg("log_activity cannot be called via CPI")]
    CpiNotAllowed,
    #[msg("Only one log_activity allowed per transaction")]
    DuplicateLogActivity,
    #[msg("Quest user does not match log_activity authority")]
    QuestUserMismatch,
    #[msg("Referral is already set and cannot be changed")]
    ReferralAlreadySet,
    #[msg("Cannot set self as referral")]
    SelfReferral,
    #[msg("Twitter verifier not configured")]
    TwitterVerifierNotSet,
    #[msg("Unauthorized: not the twitter verifier")]
    NotTwitterVerifier,
    #[msg("Twitter username too long")]
    TwitterUsernameTooLong,
    #[msg("Twitter username is empty")]
    TwitterUsernameEmpty,
    #[msg("Tweet URL too long")]
    TweetUrlTooLong,
    #[msg("Tweet URL is empty")]
    TweetUrlEmpty,
    #[msg("Twitter account is not in pending status")]
    TwitterNotPending,
    #[msg("Twitter account is not in verified status")]
    TwitterNotVerified,
    #[msg("Twitter handle already bound to another agent")]
    TwitterHandleAlreadyBound,
    #[msg("Twitter verify vault has insufficient balance")]
    InsufficientTwitterVerifyVaultBalance,
    #[msg("Tweet verification is in cooldown period")]
    TweetVerifyCooldown,
    #[msg("Tweet verification is not in pending status")]
    TweetVerifyNotPending,
    #[msg("Tweet verification already pending")]
    TweetVerifyAlreadyPending,
    #[msg("Twitter username does not match verified account")]
    TwitterUsernameMismatch,
}
