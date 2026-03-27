use anchor_lang::prelude::*;

/// Per-agent tweet verification state.
/// Seeds: [SEED_TWEET_VERIFY, agent_pda.as_ref()]
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TweetVerify {
    pub agent_id_len: u64,
    pub agent_id: [u8; 32],
    /// 0 = Idle, 1 = Pending
    pub status: u64,
    /// Unix timestamp when the tweet was submitted
    pub submitted_at: i64,
    /// Unix timestamp of the last successful reward (for cooldown)
    pub last_rewarded_at: i64,
    /// Tweet ID (Twitter snowflake ID)
    pub tweet_id: u128,
    pub _reserved: [u8; 256],
    pub _reserved2: [u8; 128],
    pub _reserved3: [u8; 64],
    pub _reserved4: [u8; 32],
    pub _reserved5: [u8; 16],
    pub _reserved6: [u8; 8],
}
