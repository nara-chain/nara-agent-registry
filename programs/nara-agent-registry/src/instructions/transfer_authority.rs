use anchor_lang::prelude::*;
use crate::state::AgentState;
use crate::error::AgentRegistryError;
use crate::seeds::*;
#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct TransferAuthority<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
}

pub fn transfer_authority(
    ctx: Context<TransferAuthority>,
    _agent_id: String,
    new_authority: Pubkey,
) -> Result<()> {
    let mut agent = ctx.accounts.agent.load_mut()?;
    require!(
        agent.pending_buffer == Pubkey::default(),
        AgentRegistryError::HasPendingBuffer
    );
    agent.authority = new_authority;
    Ok(())
}
