use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct UpdateTweetVerifyConfig<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_tweet_verify_config(
    ctx: Context<UpdateTweetVerifyConfig>,
    reward: u64,
    points: u64,
) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    config.tweet_verify_reward = reward;
    config.tweet_verify_points = points;
    Ok(())
}
