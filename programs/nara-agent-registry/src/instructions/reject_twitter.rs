use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, AgentTwitter};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::queue_remove;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RejectTwitter<'info> {
    #[account(mut)]
    pub verifier: Signer<'info>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        mut,
        seeds = [SEED_TWITTER, agent.key().as_ref()],
        bump,
    )]
    pub twitter: AccountLoader<'info, AgentTwitter>,
    /// CHECK: Global pending-verification queue PDA; managed manually.
    #[account(mut, seeds = [SEED_TWITTER_QUEUE], bump)]
    pub twitter_queue: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn reject_twitter(ctx: Context<RejectTwitter>, _agent_id: String) -> Result<()> {
    let config = ctx.accounts.config.load()?;
    require!(
        config.twitter_verifier != Pubkey::default(),
        AgentRegistryError::TwitterVerifierNotSet
    );
    require_keys_eq!(
        ctx.accounts.verifier.key(),
        config.twitter_verifier,
        AgentRegistryError::NotTwitterVerifier
    );
    drop(config);

    let mut twitter = ctx.accounts.twitter.load_mut()?;
    require!(twitter.status == 1, AgentRegistryError::TwitterNotPending);
    twitter.status = 3; // Rejected
    let twitter_key = ctx.accounts.twitter.key();
    drop(twitter);

    // Remove from pending-verification queue
    queue_remove(
        &ctx.accounts.twitter_queue.to_account_info(),
        &ctx.accounts.verifier.to_account_info(),
        &twitter_key,
    )?;

    Ok(())
}
