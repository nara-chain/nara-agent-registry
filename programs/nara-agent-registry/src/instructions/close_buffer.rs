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
    pub agent: Account<'info, AgentRecord>,
    #[account(
        mut,
        constraint = Some(buffer.key()) == agent.pending_buffer @ AgentRegistryError::BufferMismatch,
        close = authority,
    )]
    pub buffer: AccountLoader<'info, MemoryBuffer>,
    pub system_program: Program<'info, System>,
}

/// Discard the active upload buffer without finalizing.
/// The buffer account is closed (rent returned to authority) and
/// `agent.pending_buffer` is cleared, allowing a fresh upload to begin.
pub fn close_buffer(ctx: Context<CloseBuffer>, _agent_id: String) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.buffer.load()?.authority,
        ctx.accounts.authority.key(),
        AgentRegistryError::Unauthorized
    );
    ctx.accounts.agent.pending_buffer = None;
    Ok(())
}
