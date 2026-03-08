use anchor_lang::prelude::*;

/// Global program configuration. Single PDA, seeds = [SEED_CONFIG].
/// Created once by the first caller of `init_config`; that caller becomes admin.
#[account(zero_copy)]
#[repr(C)]
pub struct ProgramConfig {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub point_mint: Pubkey,
    pub referee_mint: Pubkey,
    pub referee_activity_mint: Pubkey,
    pub register_fee: u64,
    pub points_self: u64,
    pub points_referral: u64,
    pub referral_register_fee: u64,
    pub referral_fee_share: u64,
    pub referral_register_points: u64,
    pub activity_reward: u64,
    pub referral_activity_reward: u64,
    pub _reserved: [u8; 64],
}
