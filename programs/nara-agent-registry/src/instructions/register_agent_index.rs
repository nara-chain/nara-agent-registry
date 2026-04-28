use anchor_lang::prelude::*;
use crate::state::{AgentState, AgentIndex, ReverseIndex};
use crate::error::AgentRegistryError;
use crate::seeds::*;

pub const MAX_AGENT_INDEX_LEN: usize = 128;

#[derive(Accounts)]
#[instruction(index_str: String, index_hash: [u8; 32])]
pub struct RegisterAgentIndex<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    #[account(
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    /// Forward-lookup PDA: keyed by hash(index_str). Globally unique per index_str.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<AgentIndex>(),
        seeds = [SEED_AGENT_INDEX, &index_hash],
        bump,
    )]
    pub agent_index: AccountLoader<'info, AgentIndex>,
    /// Reverse-lookup PDA: keyed by (agent, hash(index_str)). One per agent per index.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<ReverseIndex>(),
        seeds = [SEED_REVERSE_INDEX, agent.key().as_ref(), &index_hash],
        bump,
    )]
    pub reverse_index: AccountLoader<'info, ReverseIndex>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent_index(
    ctx: Context<RegisterAgentIndex>,
    index_str: String,
    index_hash: [u8; 32],
) -> Result<()> {
    require!(!index_str.is_empty(), AgentRegistryError::AgentIndexEmpty);
    require!(index_str.len() <= MAX_AGENT_INDEX_LEN, AgentRegistryError::AgentIndexTooLong);

    // Verify caller-supplied hash matches the actual hash of index_str
    let computed_hash = *blake3::hash(index_str.as_bytes()).as_bytes();
    require!(computed_hash == index_hash, AgentRegistryError::AgentIndexHashMismatch);

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

    let mut rev = ctx.accounts.reverse_index.load_init()?;
    rev.agent = ctx.accounts.agent.key();
    rev.created_at = now;
    rev.index_len = index_str.len() as u32;
    rev.index = [0u8; 128];
    rev.index[..index_str.len()].copy_from_slice(index_str.as_bytes());
    drop(rev);

    msg!("register_agent_index: index={}, agent={}", index_str, std::str::from_utf8(&agent_id_buf[..agent_id_len]).unwrap_or(""));

    Ok(())
}
