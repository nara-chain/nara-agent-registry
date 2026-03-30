use anchor_lang::prelude::*;

/// Global program configuration. Single PDA, seeds = [SEED_CONFIG].
/// Created once by the first caller of `init_config`; that caller becomes admin.
#[account(zero_copy)]
#[repr(C)]
pub struct ProgramConfig {
    pub admin: Pubkey,
    pub fee_vault: Pubkey,
    pub point_mint: Pubkey,
    pub referee_mint: Pubkey,
    pub referee_activity_mint: Pubkey,
    pub register_fee: u64,
    pub points_self: u64,
    pub points_referral: u64,
    pub referral_discount_bps: u64,
    pub referral_share_bps: u64,
    pub referral_register_points: u64,
    pub activity_reward: u64,
    pub referral_activity_reward: u64,
    pub twitter_verifier: Pubkey,
    pub twitter_verification_fee: u64,
    pub twitter_verification_reward: u64,
    pub twitter_verification_points: u64,
    pub tweet_verify_reward: u64,
    pub tweet_verify_points: u64,
    pub register_fee_7: u64,
    pub register_fee_6: u64,
    pub register_fee_5: u64,
    pub _reserved: [u8; 128],
    pub _reserved2: [u8; 96],
}
