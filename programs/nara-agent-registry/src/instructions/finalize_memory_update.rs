use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program as sol_system;
use crate::state::{AgentRecord, MemoryBuffer, AgentMemory};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct FinalizeMemoryUpdate<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    #[account(
        mut,
        close = authority,
    )]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
    /// CHECK: pre-created by client (owner = this program).
    #[account(
        mut,
        owner = crate::ID @ AgentRegistryError::InvalidMemoryOwner,
    )]
    pub new_memory: UncheckedAccount<'info>,
    /// CHECK: existing AgentMemory account to close.
    #[account(mut)]
    pub old_memory: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn finalize_memory_update(ctx: Context<FinalizeMemoryUpdate>, _agent_id: String) -> Result<()> {
    let total_len;
    {
        let agent = ctx.accounts.agent.load()?;
        require_keys_eq!(
            ctx.accounts.buffer.key(),
            agent.pending_buffer,
            AgentRegistryError::BufferMismatch
        );
        require!(
            agent.memory != Pubkey::default(),
            AgentRegistryError::MemoryNotFound
        );
        require_keys_eq!(
            ctx.accounts.old_memory.key(),
            agent.memory,
            AgentRegistryError::MemoryMismatch
        );
    }

    {
        let buf = ctx.accounts.buffer.load()?;
        require_keys_eq!(buf.authority, ctx.accounts.authority.key(), AgentRegistryError::Unauthorized);
        require!(buf.write_offset == buf.total_len, AgentRegistryError::BufferIncomplete);
        total_len = buf.total_len as usize;
    }

    require!(
        ctx.accounts.new_memory.data_len() == AgentMemory::required_size(total_len),
        AgentRegistryError::InvalidMemorySize
    );

    let agent_key = ctx.accounts.agent.key();

    // Close old_memory.
    {
        let old_lamports = ctx.accounts.old_memory.lamports();
        **ctx.accounts.old_memory.try_borrow_mut_lamports()? = 0;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += old_lamports;
        ctx.accounts.old_memory.to_account_info().assign(&sol_system::ID);
        ctx.accounts.old_memory.try_borrow_mut_data()?.fill(0);
    }

    // Write new_memory.
    {
        let buf_info = ctx.accounts.buffer.to_account_info();
        let buf_data = buf_info.try_borrow_data()?;
        let slice = &buf_data[MemoryBuffer::HEADER_SIZE..MemoryBuffer::HEADER_SIZE + total_len];

        let mut nm = ctx.accounts.new_memory.try_borrow_mut_data()?;
        nm[..AgentMemory::DISC_SIZE].copy_from_slice(&AgentMemory::DISCRIMINATOR);
        nm[AgentMemory::AGENT_OFFSET..AgentMemory::AGENT_END].copy_from_slice(agent_key.as_ref());
        nm[AgentMemory::HEADER_SIZE..AgentMemory::HEADER_SIZE + total_len].copy_from_slice(slice);
    }

    let mut agent = ctx.accounts.agent.load_mut()?;
    agent.memory = ctx.accounts.new_memory.key();
    agent.pending_buffer = Pubkey::default();
    agent.version += 1;
    agent.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}
