use anchor_lang::prelude::*;
use crate::state::{ProgramConfig, AgentState, AgentTwitter, TwitterHandle};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use crate::constants::*;

#[derive(Accounts)]
#[instruction(agent_id: String, username: String)]
pub struct UnbindTwitter<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        mut,
        seeds = [SEED_TWITTER, agent.key().as_ref()],
        bump,
    )]
    pub twitter: AccountLoader<'info, AgentTwitter>,
    #[account(
        mut,
        seeds = [SEED_TWITTER_HANDLE, username.as_bytes()],
        bump,
    )]
    pub twitter_handle: AccountLoader<'info, TwitterHandle>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Twitter verify vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_TWITTER_VERIFY_VAULT], bump)]
    pub twitter_verify_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn unbind_twitter(ctx: Context<UnbindTwitter>, _agent_id: String, _username: String) -> Result<()> {
    // Verify status is Verified
    let twitter = ctx.accounts.twitter.load()?;
    require!(twitter.status == 2, AgentRegistryError::TwitterNotVerified);
    drop(twitter);

    // Verify twitter_handle.agent matches this agent
    let handle = ctx.accounts.twitter_handle.load()?;
    require_keys_eq!(
        handle.agent,
        ctx.accounts.agent.key(),
        AgentRegistryError::Unauthorized
    );
    drop(handle);

    // Pay unbind fee to twitter_verify_vault
    if UNBIND_TWITTER_FEE > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.twitter_verify_vault.to_account_info(),
                },
            ),
            UNBIND_TWITTER_FEE,
        )?;
    }

    // Clear TwitterHandle agent (keep PDA alive to record history)
    let mut handle = ctx.accounts.twitter_handle.load_mut()?;
    handle.agent = Pubkey::default();
    drop(handle);

    // Close AgentTwitter account
    let authority_info = ctx.accounts.authority.to_account_info();
    let twitter_info = ctx.accounts.twitter.to_account_info();
    **authority_info.lamports.borrow_mut() += twitter_info.lamports();
    **twitter_info.lamports.borrow_mut() = 0;
    twitter_info.assign(&anchor_lang::system_program::ID);
    twitter_info.resize(0)?;

    Ok(())
}
