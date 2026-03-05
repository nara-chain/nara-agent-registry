use anchor_lang::prelude::*;
use crate::state::{AgentRecord, MemoryBuffer};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct FinalizeMemoryAppend<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
        constraint = agent.memory != Pubkey::default() @ AgentRegistryError::MemoryNotFound,
    )]
    pub agent: Account<'info, AgentRecord>,
    #[account(
        mut,
        constraint = Some(buffer.key()) == agent.pending_buffer @ AgentRegistryError::BufferMismatch,
        close = authority,
    )]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
    /// CHECK: existing AgentMemory account to append to. Must equal agent.memory.
    /// Will be reallocated in-place to hold the additional data.
    #[account(
        mut,
        constraint = memory.key() == agent.memory @ AgentRegistryError::MemoryMismatch,
    )]
    pub memory: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Append buffer data to the end of existing memory, expanding the account via realloc.
/// No new memory account is created — the existing one grows in place.
pub fn finalize_memory_append(ctx: Context<FinalizeMemoryAppend>, _agent_id: String) -> Result<()> {
    let append_len = {
        let buf = ctx.accounts.buffer.load()?;
        require_keys_eq!(buf.authority, ctx.accounts.authority.key(), AgentRegistryError::Unauthorized);
        require!(buf.write_offset == buf.total_len, AgentRegistryError::BufferIncomplete);
        buf.total_len as usize
    };

    let memory_info = ctx.accounts.memory.to_account_info();
    let old_total = memory_info.data_len();
    let new_total = old_total + append_len;

    // Realloc memory account to fit appended data.
    memory_info.resize(new_total)?;

    // Pay for additional rent.
    let rent = Rent::get()?;
    let new_min = rent.minimum_balance(new_total);
    let current_lamports = memory_info.lamports();
    if new_min > current_lamports {
        let diff = new_min - current_lamports;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: memory_info.clone(),
                },
            ),
            diff,
        )?;
    }

    // Copy buffer data to the end of memory.
    {
        let buf_info = ctx.accounts.buffer.to_account_info();
        let buf_data = buf_info.try_borrow_data()?;
        let slice = &buf_data[MemoryBuffer::HEADER_SIZE..MemoryBuffer::HEADER_SIZE + append_len];

        let mut mem_data = memory_info.try_borrow_mut_data()?;
        mem_data[old_total..new_total].copy_from_slice(slice);
    }

    let agent = &mut ctx.accounts.agent;
    agent.pending_buffer = None;
    agent.version += 1;
    agent.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}
