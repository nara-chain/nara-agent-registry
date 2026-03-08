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

/// Default registration fee with referral (0.5 NARA).
pub const DEFAULT_REFERRAL_REGISTER_FEE: u64 = 500_000_000;

/// Default referral's share of the referral registration fee (0.25 NARA).
pub const DEFAULT_REFERRAL_FEE_SHARE: u64 = 250_000_000;

/// Default points awarded to referral agent on registration.
pub const DEFAULT_REFERRAL_REGISTER_POINTS: u64 = 10;

/// Default activity reward in lamports (0.001 SOL), transferred from treasury to user.
pub const DEFAULT_ACTIVITY_REWARD: u64 = 1_000_000;

/// Default referral activity reward in lamports (0.001 SOL), transferred from treasury to referral.
pub const DEFAULT_REFERRAL_ACTIVITY_REWARD: u64 = 1_000_000;

/// Point token name.
pub const POINT_TOKEN_NAME: &str = "NARA Point";

/// Point token symbol.
pub const POINT_TOKEN_SYMBOL: &str = "POINT";

/// Point token metadata URI (placeholder).
pub const POINT_TOKEN_URI: &str = "https://nara.build/metadata/point.json";

/// Point token decimals.
pub const POINT_TOKEN_DECIMALS: u8 = 0;

/// Referee token name.
pub const REFEREE_TOKEN_NAME: &str = "NARA Referee";

/// Referee token symbol.
pub const REFEREE_TOKEN_SYMBOL: &str = "REFEREE";

/// Referee token metadata URI.
pub const REFEREE_TOKEN_URI: &str = "https://nara.build/metadata/referee.json";

/// Referee Activity token name.
pub const REFEREE_ACTIVITY_TOKEN_NAME: &str = "NARA Referee Activity";

/// Referee Activity token symbol.
pub const REFEREE_ACTIVITY_TOKEN_SYMBOL: &str = "REFACT";

/// Referee Activity token metadata URI.
pub const REFEREE_ACTIVITY_TOKEN_URI: &str = "https://nara.build/metadata/referee-activity.json";
