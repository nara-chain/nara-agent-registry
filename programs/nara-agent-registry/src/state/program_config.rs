use anchor_lang::prelude::*;

/// Global program configuration. Single PDA, seeds = [b"config"].
/// Created once by the first caller of `init_config`; that caller becomes admin.
#[account(zero_copy)]
#[repr(C)]
pub struct ProgramConfig {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub register_fee: u64,
    pub points_self: u64,
    pub points_referral: u64,
    pub _reserved: [u8; 64],
}
