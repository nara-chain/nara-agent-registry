use anchor_lang::prelude::*;
use crate::state::{AgentRecord, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::constants::{MIN_AGENT_ID_LEN, MAX_AGENT_ID_LEN};

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RegisterAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AgentRecord>(),
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    #[account(
        seeds = [b"config"],
        bump,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: must equal config.fee_recipient; validated in handler.
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
    require!(agent_id.len() >= MIN_AGENT_ID_LEN, AgentRegistryError::AgentIdTooShort);
    require!(agent_id.len() <= MAX_AGENT_ID_LEN, AgentRegistryError::AgentIdTooLong);

    let config = ctx.accounts.config.load()?;
    let fee = config.register_fee;
    require_keys_eq!(
        ctx.accounts.fee_recipient.key(),
        config.fee_recipient,
        AgentRegistryError::InvalidFeeRecipient
    );
    drop(config);

    if fee > 0 && ctx.accounts.fee_recipient.key() != ctx.accounts.authority.key() {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.fee_recipient.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    let now = Clock::get()?.unix_timestamp;
    let mut agent = ctx.accounts.agent.load_init()?;
    agent.authority = ctx.accounts.authority.key();
    agent.agent_id_len = agent_id.len() as u32;
    agent.agent_id[..agent_id.len()].copy_from_slice(agent_id.as_bytes());
    agent.pending_buffer = Pubkey::default();
    agent.memory = Pubkey::default();
    agent.points = 0;
    agent.version = 0;
    agent.created_at = now;
    agent.updated_at = 0;
    Ok(())
}
