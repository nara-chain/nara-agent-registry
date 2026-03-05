use anchor_lang::prelude::*;
use crate::state::{AgentRecord, MemoryBuffer};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct WriteToBuffer<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    #[account(mut)]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
}

pub fn write_to_buffer(
    ctx: Context<WriteToBuffer>,
    _agent_id: String,
    offset: u32,
    data: Vec<u8>,
) -> Result<()> {
    // Validate buffer matches agent's pending_buffer.
    {
        let agent = ctx.accounts.agent.load()?;
        require_keys_eq!(
            ctx.accounts.buffer.key(),
            agent.pending_buffer,
            AgentRegistryError::BufferMismatch
        );
    }

    {
        let buf = ctx.accounts.buffer.load()?;
        require_keys_eq!(buf.authority, ctx.accounts.authority.key(), AgentRegistryError::Unauthorized);
        require!(offset == buf.write_offset, AgentRegistryError::OffsetMismatch);
        require!(
            offset as usize + data.len() <= buf.total_len as usize,
            AgentRegistryError::WriteOutOfBounds
        );
    }

    {
        let buf_info = ctx.accounts.buffer.to_account_info();
        let mut buf_data = buf_info.try_borrow_mut_data()?;
        let start = MemoryBuffer::HEADER_SIZE + offset as usize;
        buf_data[start..start + data.len()].copy_from_slice(&data);
    }

    ctx.accounts.buffer.load_mut()?.write_offset += data.len() as u32;
    Ok(())
}
