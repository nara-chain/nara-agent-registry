use anchor_lang::prelude::*;
use crate::state::{AgentRecord, MemoryBuffer, AgentMemory};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct FinalizeMemoryNew<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: Account<'info, AgentRecord>,
    #[account(
        mut,
        constraint = Some(buffer.key()) == agent.pending_buffer @ AgentRegistryError::BufferMismatch,
        close = authority,
    )]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
    /// CHECK: pre-created by the client (owner = this program,
    /// space = AgentMemory::required_size(total_len)).
    /// This instruction writes the discriminator + header + memory bytes.
    #[account(
        mut,
        owner = crate::ID @ AgentRegistryError::InvalidMemoryOwner,
    )]
    pub new_memory: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Finalise a buffer upload for an agent that has **no existing memory**.
pub fn finalize_memory_new(ctx: Context<FinalizeMemoryNew>, _agent_id: String) -> Result<()> {
    let total_len = {
        let buf = ctx.accounts.buffer.load()?;
        require_keys_eq!(buf.authority, ctx.accounts.authority.key(), AgentRegistryError::Unauthorized);
        require!(buf.write_offset == buf.total_len, AgentRegistryError::BufferIncomplete);
        buf.total_len as usize
    };

    require!(
        ctx.accounts.agent.memory == Pubkey::default(),
        AgentRegistryError::MemoryAlreadyExists
    );
    require!(
        ctx.accounts.new_memory.data_len() == AgentMemory::required_size(total_len),
        AgentRegistryError::InvalidMemorySize
    );

    let agent_key = ctx.accounts.agent.key();

    {
        let buf_info = ctx.accounts.buffer.to_account_info();
        let buf_data = buf_info.try_borrow_data()?;
        let slice = &buf_data[MemoryBuffer::HEADER_SIZE..MemoryBuffer::HEADER_SIZE + total_len];

        let mut nm = ctx.accounts.new_memory.try_borrow_mut_data()?;
        nm[..8].copy_from_slice(&AgentMemory::DISCRIMINATOR);
        nm[8..40].copy_from_slice(agent_key.as_ref());
        nm[40..40 + total_len].copy_from_slice(slice);
    }

    let agent = &mut ctx.accounts.agent;
    agent.memory = ctx.accounts.new_memory.key();
    agent.pending_buffer = None;
    agent.version = 1;
    agent.updated_at = Clock::get()?.unix_timestamp;
    Ok(())
}
