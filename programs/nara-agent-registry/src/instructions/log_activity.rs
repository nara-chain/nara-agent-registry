use anchor_lang::prelude::*;
use crate::state::AgentRecord;
use crate::error::AgentRegistryError;

#[event]
pub struct ActivityLogged {
    pub agent_id: String,
    pub authority: Pubkey,
    pub model: String,
    pub activity: String,
    pub log: String,
    pub timestamp: i64,
}

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct LogActivity<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: Account<'info, AgentRecord>,
}

pub fn log_activity(
    ctx: Context<LogActivity>,
    agent_id: String,
    model: String,
    activity: String,
    log: String,
) -> Result<()> {
    let clock = Clock::get()?;

    emit!(ActivityLogged {
        agent_id,
        authority: ctx.accounts.authority.key(),
        model,
        activity,
        log,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
