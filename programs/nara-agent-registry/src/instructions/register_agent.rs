use anchor_lang::prelude::*;
use crate::state::{AgentRecord, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::MIN_AGENT_ID_LEN;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RegisterAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = AgentRecord::space(agent_id.len()),
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
    )]
    pub agent: Account<'info, AgentRecord>,
    #[account(
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, ProgramConfig>,
    /// CHECK: must equal config.fee_recipient; validated by constraint below.
    #[account(
        mut,
        constraint = fee_recipient.key() == config.fee_recipient @ AgentRegistryError::InvalidFeeRecipient,
    )]
    pub fee_recipient: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
    require!(agent_id.len() >= MIN_AGENT_ID_LEN, AgentRegistryError::AgentIdTooShort);

    let fee = ctx.accounts.config.register_fee;
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
    let agent = &mut ctx.accounts.agent;
    agent.authority = ctx.accounts.authority.key();
    agent.agent_id = agent_id;
    agent.pending_buffer = None;
    agent.memory = Pubkey::default();
    agent.version = 0;
    agent.created_at = now;
    agent.updated_at = 0;
    Ok(())
}
