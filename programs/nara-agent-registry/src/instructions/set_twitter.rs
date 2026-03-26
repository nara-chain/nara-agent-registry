use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, AgentTwitter, TwitterQueue};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use crate::constants::*;
use super::helpers::{queue_push};

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct SetTwitter<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + std::mem::size_of::<AgentTwitter>(),
        seeds = [SEED_TWITTER, agent.key().as_ref()],
        bump,
    )]
    pub twitter: AccountLoader<'info, AgentTwitter>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Twitter verify vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_TWITTER_VERIFY_VAULT], bump)]
    pub twitter_verify_vault: UncheckedAccount<'info>,
    /// CHECK: Global pending-verification queue PDA; managed manually.
    #[account(mut, seeds = [SEED_TWITTER_QUEUE], bump)]
    pub twitter_queue: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn set_twitter(ctx: Context<SetTwitter>, _agent_id: String, username: String, tweet_url: String) -> Result<()> {
    require!(!username.is_empty(), AgentRegistryError::TwitterUsernameEmpty);
    require!(username.len() <= MAX_TWITTER_USERNAME_LEN, AgentRegistryError::TwitterUsernameTooLong);
    require!(!tweet_url.is_empty(), AgentRegistryError::TweetUrlEmpty);
    require!(tweet_url.len() <= MAX_TWEET_URL_LEN, AgentRegistryError::TweetUrlTooLong);

    let config = ctx.accounts.config.load()?;
    let fee = config.twitter_verification_fee;
    drop(config);

    // Pay verification fee
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

    // For init_if_needed with AccountLoader (zero_copy): if the account was just
    // created, discriminator is still zeros — use load_init() to write it.
    // If it already exists, use load_mut() which verifies the discriminator.
    let is_new = {
        let acc_info = ctx.accounts.twitter.to_account_info();
        let data = acc_info.try_borrow_data()?;
        data[..8] == [0u8; 8]
    };
    let mut twitter = if is_new {
        ctx.accounts.twitter.load_init()?
    } else {
        ctx.accounts.twitter.load_mut()?
    };
    twitter.agent_id_len = agent_id.len() as u64;
    twitter.agent_id = [0u8; 32];
    twitter.agent_id[..agent_id.len()].copy_from_slice(agent_id.as_bytes());
    twitter.status = 1; // Pending
    twitter.verified_at = 0;
    twitter.username_len = username.len() as u64;
    twitter.username = [0u8; 32];
    twitter.username[..username.len()].copy_from_slice(username.as_bytes());
    twitter.tweet_url_len = tweet_url.len() as u64;
    twitter.tweet_url = [0u8; 256];
    twitter.tweet_url[..tweet_url.len()].copy_from_slice(tweet_url.as_bytes());
    let twitter_key = ctx.accounts.twitter.key();
    drop(twitter);

    // Add this twitter PDA to the global pending-verification queue
    queue_push(
        &ctx.accounts.twitter_queue.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.program_id,
        &[SEED_TWITTER_QUEUE],
        &twitter_key,
        &TwitterQueue::DISCRIMINATOR,
    )?;

    Ok(())
}
