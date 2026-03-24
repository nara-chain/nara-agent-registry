use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct UpdateTwitterVerificationConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_twitter_verification_config(
    ctx: Context<UpdateTwitterVerificationConfig>,
    fee: u64,
    reward: u64,
    points: u64,
) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    config.twitter_verification_fee = fee;
    config.twitter_verification_reward = reward;
    config.twitter_verification_points = points;
    Ok(())
}
