use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentIndex, AgentAlias};
use crate::error::AgentRegistryError;
use crate::seeds::*;

pub const MAX_AGENT_INDEX_LEN: usize = 32;

#[derive(Accounts)]
#[instruction(index_str: String)]
pub struct RegisterAgentIndex<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<AgentIndex>(),
        seeds = [SEED_AGENT_INDEX, index_str.as_bytes()],
        bump,
    )]
    pub agent_index: AccountLoader<'info, AgentIndex>,
    /// Reverse-lookup PDA: given the agent, list all aliases via getProgramAccounts.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<AgentAlias>(),
        seeds = [SEED_AGENT_ALIAS, agent.key().as_ref(), index_str.as_bytes()],
        bump,
    )]
    pub agent_alias: AccountLoader<'info, AgentAlias>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent_index(ctx: Context<RegisterAgentIndex>, index_str: String) -> Result<()> {
    require!(!index_str.is_empty(), AgentRegistryError::AgentIndexEmpty);
    require!(index_str.len() <= MAX_AGENT_INDEX_LEN, AgentRegistryError::AgentIndexTooLong);

    let agent = ctx.accounts.agent.load()?;
    let agent_id_len = agent.agent_id_len as usize;
    let mut agent_id_buf = [0u8; 32];
    agent_id_buf[..agent_id_len].copy_from_slice(&agent.agent_id[..agent_id_len]);
    drop(agent);

    let now = Clock::get()?.unix_timestamp;

    let mut idx = ctx.accounts.agent_index.load_init()?;
    idx.agent = ctx.accounts.agent.key();
    idx.created_at = now;
    idx.agent_id_len = agent_id_len as u32;
    idx.agent_id = agent_id_buf;
    drop(idx);

    let mut alias = ctx.accounts.agent_alias.load_init()?;
    alias.agent = ctx.accounts.agent.key();
    alias.created_at = now;
    alias.index_len = index_str.len() as u32;
    alias.index = [0u8; 32];
    alias.index[..index_str.len()].copy_from_slice(index_str.as_bytes());
    drop(alias);

    msg!("register_agent_index: index={}, agent={}", index_str, std::str::from_utf8(&agent_id_buf[..agent_id_len]).unwrap_or(""));

    Ok(())
}
