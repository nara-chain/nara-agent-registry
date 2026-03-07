use anchor_lang::prelude::*;
use crate::state::AgentState;
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::close_raw_account;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct DeleteAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
        close = authority,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    /// CHECK: AgentBio PDA (seeds = [SEED_BIO, agent]).
    #[account(
        mut,
        seeds = [SEED_BIO, agent.key().as_ref()],
        bump,
    )]
    pub bio: UncheckedAccount<'info>,
    /// CHECK: AgentMetadata PDA (seeds = [SEED_META, agent]).
    #[account(
        mut,
        seeds = [SEED_META, agent.key().as_ref()],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: AgentMemory account.
    #[account(mut)]
    pub memory_account: UncheckedAccount<'info>,
}

pub fn delete_agent(ctx: Context<DeleteAgent>, _agent_id: String) -> Result<()> {
    {
        let agent = ctx.accounts.agent.load()?;
        require!(
            agent.pending_buffer == Pubkey::default(),
            AgentRegistryError::HasPendingBuffer
        );

        if agent.memory != Pubkey::default() {
            require_keys_eq!(
                ctx.accounts.memory_account.key(),
                agent.memory,
                AgentRegistryError::MemoryMismatch
            );
        }
    }

    // Close AgentMemory if present.
    if ctx.accounts.agent.load()?.memory != Pubkey::default() {
        close_raw_account(
            &ctx.accounts.memory_account.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    if ctx.accounts.bio.lamports() > 0 {
        close_raw_account(
            &ctx.accounts.bio.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    if ctx.accounts.metadata.lamports() > 0 {
        close_raw_account(
            &ctx.accounts.metadata.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    Ok(())
}
