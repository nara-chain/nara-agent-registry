use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentMetadata};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::write_dynamic_pda;

#[derive(Accounts)]
#[instruction(agent_id: String, data: String)]
pub struct SetMetadata<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    /// CHECK: AgentMetadata PDA — created or resized in the handler.
    #[account(
        mut,
        seeds = [SEED_META, agent.key().as_ref()],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn set_metadata(ctx: Context<SetMetadata>, _agent_id: String, data: String) -> Result<()> {
    let agent_key = ctx.accounts.agent.key();
    let pda_seeds: &[&[u8]] = &[SEED_META, agent_key.as_ref()];

    write_dynamic_pda(
        &ctx.accounts.metadata.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        pda_seeds,
        &AgentMetadata::DISCRIMINATOR,
        AgentMetadata::HEADER_SIZE,
        data.as_bytes(),
        ctx.program_id,
    )
}
