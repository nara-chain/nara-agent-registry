use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct UpdateActivityConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_activity_config(
    ctx: Context<UpdateActivityConfig>,
    activity_reward: u64,
    referral_activity_reward: u64,
) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    config.activity_reward = activity_reward;
    config.referral_activity_reward = referral_activity_reward;
    Ok(())
}
