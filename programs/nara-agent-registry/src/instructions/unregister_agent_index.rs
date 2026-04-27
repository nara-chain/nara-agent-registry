use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentIndex, AgentAlias};
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
    /// Reverse-lookup PDA, closed alongside the main index entry.
    #[account(
        mut,
        close = rent_destination,
        seeds = [SEED_AGENT_ALIAS, agent.key().as_ref(), index_str.as_bytes()],
        bump,
    )]
    pub agent_alias: AccountLoader<'info, AgentAlias>,
    pub system_program: Program<'info, System>,
}

pub fn unregister_agent_index(_ctx: Context<UnregisterAgentIndex>, index_str: String) -> Result<()> {
    msg!("unregister_agent_index: index={}", index_str);
    Ok(())
}
