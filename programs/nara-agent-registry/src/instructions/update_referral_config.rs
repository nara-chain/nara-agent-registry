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
    referral_register_fee: u64,
    referral_fee_share: u64,
    referral_register_points: u64,
) -> Result<()> {
    require!(
        referral_fee_share <= referral_register_fee,
        AgentRegistryError::InvalidReferralFeeConfig
    );
    let mut config = ctx.accounts.config.load_mut()?;
    config.referral_register_fee = referral_register_fee;
    config.referral_fee_share = referral_fee_share;
    config.referral_register_points = referral_register_points;
    Ok(())
}
