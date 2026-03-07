use anchor_lang::prelude::*;
use crate::state::AgentState;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct SetReferral<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    pub referral_agent: AccountLoader<'info, AgentState>,
}

pub fn set_referral(ctx: Context<SetReferral>, _agent_id: String) -> Result<()> {
    let mut agent = ctx.accounts.agent.load_mut()?;

    require!(agent.referral_id_len == 0, AgentRegistryError::ReferralAlreadySet);

    // Prevent self-referral
    require!(
        ctx.accounts.referral_agent.key() != ctx.accounts.agent.key(),
        AgentRegistryError::SelfReferral
    );

    let referral = ctx.accounts.referral_agent.load()?;
    let rid_len = referral.agent_id_len as usize;
    agent.referral_id_len = rid_len as u32;
    agent.referral_id[..rid_len].copy_from_slice(&referral.agent_id[..rid_len]);

    Ok(())
}
