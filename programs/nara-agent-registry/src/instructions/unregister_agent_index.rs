use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentIndex};
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
#[instruction(index_str: String)]
pub struct UnregisterAgentIndex<'info> {
    /// CHECK: Receives rent refund (typically the original payer or authority)
    #[account(mut)]
    pub rent_destination: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
    #[account(
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        mut,
        close = rent_destination,
        seeds = [SEED_AGENT_INDEX, index_str.as_bytes()],
        bump,
        constraint = agent_index.load()?.agent == agent.key() @ AgentRegistryError::AgentIndexMismatch,
    )]
    pub agent_index: AccountLoader<'info, AgentIndex>,
    pub system_program: Program<'info, System>,
}

pub fn unregister_agent_index(_ctx: Context<UnregisterAgentIndex>, index_str: String) -> Result<()> {
    msg!("unregister_agent_index: index={}", index_str);
    Ok(())
}
