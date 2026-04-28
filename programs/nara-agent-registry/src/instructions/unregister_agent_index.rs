use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentIndex, ReverseIndex};
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
#[instruction(index_hash: [u8; 32])]
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
        seeds = [SEED_AGENT_INDEX, &index_hash],
        bump,
        constraint = agent_index.load()?.agent == agent.key() @ AgentRegistryError::AgentIndexMismatch,
    )]
    pub agent_index: AccountLoader<'info, AgentIndex>,
    /// Reverse-lookup PDA, closed alongside the main index entry.
    #[account(
        mut,
        close = rent_destination,
        seeds = [SEED_REVERSE_INDEX, agent.key().as_ref(), &index_hash],
        bump,
    )]
    pub reverse_index: AccountLoader<'info, ReverseIndex>,
    pub system_program: Program<'info, System>,
}

pub fn unregister_agent_index(_ctx: Context<UnregisterAgentIndex>, _index_hash: [u8; 32]) -> Result<()> {
    msg!("unregister_agent_index");
    Ok(())
}
