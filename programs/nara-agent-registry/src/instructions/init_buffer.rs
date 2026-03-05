use anchor_lang::prelude::*;
use crate::state::{AgentRecord, MemoryBuffer};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct InitBuffer<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: Account<'info, AgentRecord>,
    /// Pre-created by the client (owner = this program, data all zeros).
    /// load_init() writes the discriminator + header fields.
    #[account(zero)]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
}

pub fn init_buffer(ctx: Context<InitBuffer>, _agent_id: String, total_len: u32) -> Result<()> {
    require!(
        ctx.accounts.agent.pending_buffer.is_none(),
        AgentRegistryError::PendingBufferExists
    );
    require!(
        ctx.accounts.buffer.to_account_info().data_len()
            == MemoryBuffer::required_size(total_len as usize),
        AgentRegistryError::InvalidBufferSize
    );

    {
        let mut buf = ctx.accounts.buffer.load_init()?;
        buf.authority = ctx.accounts.authority.key();
        buf.agent = ctx.accounts.agent.key();
        buf.total_len = total_len;
        buf.write_offset = 0;
    }

    ctx.accounts.agent.pending_buffer = Some(ctx.accounts.buffer.key());
    Ok(())
}
