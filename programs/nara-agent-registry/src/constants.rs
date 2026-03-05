/// Minimum agent ID length in bytes.
pub const MIN_AGENT_ID_LEN: usize = 5;

/// Maximum agent ID length in bytes (must fit in [u8; 32]).
pub const MAX_AGENT_ID_LEN: usize = 32;

/// Default registration fee in lamports (1 NARA).
pub const DEFAULT_REGISTER_FEE: u64 = 1_000_000_000;

/// Points awarded to the agent itself when logging activity with a valid quest.
pub const POINTS_SELF: u64 = 10;

/// Points awarded to the referral agent when logging activity with a valid quest.
pub const POINTS_REFERRAL: u64 = 1;
