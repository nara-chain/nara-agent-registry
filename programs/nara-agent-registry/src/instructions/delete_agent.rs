use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program as sol_system;
use crate::state::AgentRecord;
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct DeleteAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// AgentRecord PDA — closed by Anchor after the handler returns.
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
        close = authority,
    )]
    pub agent: Account<'info, AgentRecord>,
    /// CHECK: AgentBio PDA (seeds = [b"bio", agent]).
    ///        Closed inside the handler if it has been created.
    #[account(
        mut,
        seeds = [b"bio", agent.key().as_ref()],
        bump,
    )]
    pub bio: UncheckedAccount<'info>,
    /// CHECK: AgentMetadata PDA (seeds = [b"meta", agent]).
    ///        Closed inside the handler if it has been created.
    #[account(
        mut,
        seeds = [b"meta", agent.key().as_ref()],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: AgentMemory account. Must equal agent.memory when agent has memory.
    ///        Pass any account (e.g. authority) when agent has no memory.
    #[account(mut)]
    pub memory_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Delete an agent, closing all associated accounts and returning rent to the authority.
/// Requires no pending buffer — call `close_buffer` first if one exists.
/// After deletion the agent_id can be re-registered.
pub fn delete_agent(ctx: Context<DeleteAgent>, _agent_id: String) -> Result<()> {
    require!(
        ctx.accounts.agent.pending_buffer.is_none(),
        AgentRegistryError::HasPendingBuffer
    );

    // Close AgentMemory if the agent has memory.
    if ctx.accounts.agent.memory != Pubkey::default() {
        require_keys_eq!(
            ctx.accounts.memory_account.key(),
            ctx.accounts.agent.memory,
            AgentRegistryError::MemoryMismatch
        );
        close_raw_account(
            &ctx.accounts.memory_account.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    // Close AgentBio if it has been created (lamports > 0).
    if ctx.accounts.bio.lamports() > 0 {
        close_raw_account(
            &ctx.accounts.bio.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    // Close AgentMetadata if it has been created (lamports > 0).
    if ctx.accounts.metadata.lamports() > 0 {
        close_raw_account(
            &ctx.accounts.metadata.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
        )?;
    }

    // AgentRecord is closed by the `close = authority` constraint after this handler returns.
    Ok(())
}

/// Drain lamports, zero data, and reassign owner to system program.
fn close_raw_account(account: &AccountInfo, destination: &AccountInfo) -> Result<()> {
    let lamports = account.lamports();
    **account.try_borrow_mut_lamports()? = 0;
    **destination.try_borrow_mut_lamports()? += lamports;
    account.assign(&sol_system::ID);
    account.try_borrow_mut_data()?.fill(0);
    Ok(())
}
