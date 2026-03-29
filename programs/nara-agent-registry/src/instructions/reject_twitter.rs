use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, AgentTwitter, TwitterQueue};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::queue_remove;
use super::verify_twitter::TwitterBindResult;

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
    let username_len = twitter.username_len as usize;
    let username = String::from_utf8_lossy(&twitter.username[..username_len]).to_string();
    let twitter_key = ctx.accounts.twitter.key();
    let agent = ctx.accounts.agent.load()?;
    let authority = agent.authority;
    drop(agent);
    drop(twitter);

    // Remove from pending-verification queue
    queue_remove(
        &ctx.accounts.twitter_queue.to_account_info(),
        &ctx.accounts.verifier.to_account_info(),
        &twitter_key,
        &TwitterQueue::DISCRIMINATOR,
    )?;

    msg!("reject_twitter: agent={}, username={}, approved=false", _agent_id, username);

    emit!(TwitterBindResult {
        agent_id: _agent_id,
        authority,
        username,
        approved: false,
        fee_refunded: 0,
        reward: 0,
        points: 0,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
