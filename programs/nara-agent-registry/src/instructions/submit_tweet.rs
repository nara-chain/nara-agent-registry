use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, AgentTwitter, TweetVerify, TweetVerifyQueue};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use crate::constants::*;
use super::helpers::queue_push;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct SubmitTweet<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        seeds = [SEED_TWITTER, agent.key().as_ref()],
        bump,
    )]
    pub twitter: AccountLoader<'info, AgentTwitter>,
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + std::mem::size_of::<TweetVerify>(),
        seeds = [SEED_TWEET_VERIFY, agent.key().as_ref()],
        bump,
    )]
    pub tweet_verify: AccountLoader<'info, TweetVerify>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Twitter verify vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_TWITTER_VERIFY_VAULT], bump)]
    pub twitter_verify_vault: UncheckedAccount<'info>,
    /// CHECK: Tweet verify queue PDA; managed manually.
    #[account(mut, seeds = [SEED_TWEET_VERIFY_QUEUE], bump)]
    pub tweet_verify_queue: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn submit_tweet(ctx: Context<SubmitTweet>, _agent_id: String, username: String, tweet_url: String) -> Result<()> {
    require!(!tweet_url.is_empty(), AgentRegistryError::TweetUrlEmpty);
    require!(tweet_url.len() <= MAX_TWEET_URL_LEN, AgentRegistryError::TweetUrlTooLong);

    // Verify agent has verified twitter
    let twitter = ctx.accounts.twitter.load()?;
    require!(twitter.status == 2, AgentRegistryError::TwitterNotVerified);

    // Verify username matches stored twitter username
    let stored_len = twitter.username_len as usize;
    let stored_username = &twitter.username[..stored_len];
    require!(
        username.as_bytes() == stored_username,
        AgentRegistryError::TwitterUsernameMismatch
    );
    drop(twitter);

    // Load or init TweetVerify
    let is_new = {
        let acc_info = ctx.accounts.tweet_verify.to_account_info();
        let data = acc_info.try_borrow_data()?;
        data[..8] == [0u8; 8]
    };
    let mut tv = if is_new {
        ctx.accounts.tweet_verify.load_init()?
    } else {
        let tv = ctx.accounts.tweet_verify.load_mut()?;
        // Must be idle (not pending)
        require!(tv.status == 0, AgentRegistryError::TweetVerifyAlreadyPending);
        // Check cooldown
        if tv.last_rewarded_at > 0 {
            let now = Clock::get()?.unix_timestamp;
            require!(
                now >= tv.last_rewarded_at + TWEET_VERIFY_COOLDOWN,
                AgentRegistryError::TweetVerifyCooldown
            );
        }
        tv
    };

    // Pay verification fee
    let config = ctx.accounts.config.load()?;
    let fee = config.twitter_verification_fee;
    drop(config);

    if fee > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.twitter_verify_vault.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    // Update TweetVerify
    tv.status = 1; // Pending
    tv.submitted_at = Clock::get()?.unix_timestamp;
    tv.tweet_url_len = tweet_url.len() as u64;
    tv.tweet_url = [0u8; 256];
    tv.tweet_url[..tweet_url.len()].copy_from_slice(tweet_url.as_bytes());
    let tv_key = ctx.accounts.tweet_verify.key();
    drop(tv);

    // Add to queue
    queue_push(
        &ctx.accounts.tweet_verify_queue.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
        &[SEED_TWEET_VERIFY_QUEUE],
        &tv_key,
        &TweetVerifyQueue::DISCRIMINATOR,
    )?;

    Ok(())
}
