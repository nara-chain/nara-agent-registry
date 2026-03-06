/// Minimum agent ID length in bytes.
pub const MIN_AGENT_ID_LEN: usize = 5;

/// Maximum agent ID length in bytes (must fit in [u8; 32]).
pub const MAX_AGENT_ID_LEN: usize = 32;

/// Default registration fee in lamports (1 NARA).
pub const DEFAULT_REGISTER_FEE: u64 = 1_000_000_000;

/// Default points awarded to the agent itself per valid quest.
pub const DEFAULT_POINTS_SELF: u64 = 10;

/// Default points awarded to the referral agent per valid quest.
pub const DEFAULT_POINTS_REFERRAL: u64 = 1;
