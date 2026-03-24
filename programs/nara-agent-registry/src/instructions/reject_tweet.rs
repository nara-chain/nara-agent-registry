use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, TweetVerify, TweetVerifyQueue};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::queue_remove;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RejectTweet<'info> {
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
        seeds = [SEED_TWEET_VERIFY, agent.key().as_ref()],
        bump,
    )]
    pub tweet_verify: AccountLoader<'info, TweetVerify>,
    /// CHECK: Tweet verify queue PDA; managed manually.
    #[account(mut, seeds = [SEED_TWEET_VERIFY_QUEUE], bump)]
    pub tweet_verify_queue: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn reject_tweet(ctx: Context<RejectTweet>, _agent_id: String) -> Result<()> {
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

    let mut tv = ctx.accounts.tweet_verify.load_mut()?;
    require!(tv.status == 1, AgentRegistryError::TweetVerifyNotPending);
    tv.status = 0; // Idle — no refund
    let tv_key = ctx.accounts.tweet_verify.key();
    drop(tv);

    // Remove from queue
    queue_remove(
        &ctx.accounts.tweet_verify_queue.to_account_info(),
        &ctx.accounts.verifier.to_account_info(),
        &tv_key,
        &TweetVerifyQueue::DISCRIMINATOR,
    )?;

    Ok(())
}
