use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;
#[derive(Accounts)]
pub struct UpdateReferralConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_referral_config(
    ctx: Context<UpdateReferralConfig>,
    referral_discount_bps: u64,
    referral_share_bps: u64,
    referral_register_points: u64,
) -> Result<()> {
    require!(
        referral_discount_bps <= 10_000,
        AgentRegistryError::InvalidReferralFeeConfig
    );
    require!(
        referral_share_bps <= 10_000,
        AgentRegistryError::InvalidReferralFeeConfig
    );
    let mut config = ctx.accounts.config.load_mut()?;
    config.referral_discount_bps = referral_discount_bps;
    config.referral_share_bps = referral_share_bps;
    config.referral_register_points = referral_register_points;
    Ok(())
}
