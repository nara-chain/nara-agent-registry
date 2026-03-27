use anchor_lang::prelude::*;

/// Records an approved tweet to prevent duplicate submissions.
/// Seeds: [SEED_TWEET_RECORD, &tweet_id.to_le_bytes()]
#[account(zero_copy)]
#[repr(C, packed)]
pub struct TweetRecord {
    /// The agent PDA that submitted this tweet
    pub agent: Pubkey,
    /// Unix timestamp when this tweet was approved
    pub approved_at: i64,
    /// Tweet ID (Twitter snowflake ID)
    pub tweet_id: u128,
    pub _reserved: [u8; 64],
}
