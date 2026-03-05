use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program as sol_system;
use crate::state::AgentRecord;
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct DeleteAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
        close = authority,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    /// CHECK: AgentBio PDA (seeds = [b"bio", agent]).
    #[account(
        mut,
        seeds = [b"bio", agent.key().as_ref()],
        bump,
    )]
    pub bio: UncheckedAccount<'info>,
    /// CHECK: AgentMetadata PDA (seeds = [b"meta", agent]).
    #[account(
        mut,
        seeds = [b"meta", agent.key().as_ref()],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: AgentMemory account.
    #[account(mut)]
    pub memory_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
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

fn close_raw_account(account: &AccountInfo, destination: &AccountInfo) -> Result<()> {
    let lamports = account.lamports();
    **account.try_borrow_mut_lamports()? = 0;
    **destination.try_borrow_mut_lamports()? += lamports;
    account.assign(&sol_system::ID);
    account.try_borrow_mut_data()?.fill(0);
    Ok(())
}
