use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentBio};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::write_dynamic_pda;

#[derive(Accounts)]
#[instruction(agent_id: String, bio: String)]
pub struct SetBio<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    /// CHECK: AgentBio PDA — created or resized in the handler.
    #[account(
        mut,
        seeds = [SEED_BIO, agent.key().as_ref()],
        bump,
    )]
    pub bio_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn set_bio(
    ctx: Context<SetBio>,
    _agent_id: String,
    bio: String,
) -> Result<()> {
    let agent_key = ctx.accounts.agent.key();
    let pda_seeds: &[&[u8]] = &[SEED_BIO, agent_key.as_ref()];

    write_dynamic_pda(
        &ctx.accounts.bio_account.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        pda_seeds,
        &AgentBio::DISCRIMINATOR,
        AgentBio::HEADER_SIZE,
        bio.as_bytes(),
        ctx.program_id,
    )
}
