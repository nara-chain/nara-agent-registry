use anchor_lang::prelude::*;
use crate::state::AgentRecord;
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct TransferAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: Account<'info, AgentRecord>,
}

pub fn transfer_authority(
    ctx: Context<TransferAuthority>,
    _agent_id: String,
    new_authority: Pubkey,
) -> Result<()> {
    require!(
        ctx.accounts.agent.pending_buffer.is_none(),
        AgentRegistryError::HasPendingBuffer
    );
    ctx.accounts.agent.authority = new_authority;
    Ok(())
}
