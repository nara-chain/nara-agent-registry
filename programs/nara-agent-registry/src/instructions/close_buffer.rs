use anchor_lang::prelude::*;
use crate::state::{AgentRecord, MemoryBuffer};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct CloseBuffer<'info> {
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
    pub system_program: Program<'info, System>,
}

pub fn close_buffer(ctx: Context<CloseBuffer>, _agent_id: String) -> Result<()> {
    {
        let agent = ctx.accounts.agent.load()?;
        require_keys_eq!(
            ctx.accounts.buffer.key(),
            agent.pending_buffer,
            AgentRegistryError::BufferMismatch
        );
    }
    require_keys_eq!(
        ctx.accounts.buffer.load()?.authority,
        ctx.accounts.authority.key(),
        AgentRegistryError::Unauthorized
    );
    ctx.accounts.agent.load_mut()?.pending_buffer = Pubkey::default();
    Ok(())
}
