use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;
#[derive(Accounts)]
pub struct UpdatePointsConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_points_config(
    ctx: Context<UpdatePointsConfig>,
    points_self: u64,
    points_referral: u64,
) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    config.points_self = points_self;
    config.points_referral = points_referral;
    Ok(())
}
