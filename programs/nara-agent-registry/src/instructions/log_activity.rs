use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use crate::state::AgentRecord;
use crate::error::AgentRegistryError;
use crate::constants::{POINTS_SELF, POINTS_REFERRAL};
use crate::nara_quest;

#[event]
pub struct ActivityLogged {
    pub agent_id: String,
    pub authority: Pubkey,
    pub model: String,
    pub activity: String,
    pub log: String,
    pub referral_id: String,
    pub timestamp: i64,
}

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct LogActivity<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    /// Optional referral agent PDA. Pass null if no referral.
    #[account(mut)]
    pub referral_agent: Option<AccountLoader<'info, AgentRecord>>,
    /// CHECK: Instructions sysvar for verifying submit_answer ix in tx.
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn log_activity(
    ctx: Context<LogActivity>,
    agent_id: String,
    model: String,
    activity: String,
    log: String,
) -> Result<()> {
    let clock = Clock::get()?;

    let has_quest_ix = has_submit_answer_ix(&ctx.accounts.instructions.to_account_info())?;

    let referral_id = if let Some(ref referral_loader) = ctx.accounts.referral_agent {
        let r = referral_loader.load()?;
        r.agent_id_str().to_string()
    } else {
        String::new()
    };

    if has_quest_ix {
        let mut agent = ctx.accounts.agent.load_mut()?;
        agent.points += POINTS_SELF;
        drop(agent);

        if let Some(ref referral_loader) = ctx.accounts.referral_agent {
            let mut referral_record = referral_loader.load_mut()?;
            referral_record.points += POINTS_REFERRAL;
        }
    }

    emit!(ActivityLogged {
        agent_id,
        authority: ctx.accounts.authority.key(),
        model,
        activity,
        log,
        referral_id,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

fn has_submit_answer_ix(instructions_account: &AccountInfo) -> Result<bool> {
    let mut idx = 0u16;
    loop {
        match ix_sysvar::load_instruction_at_checked(idx as usize, instructions_account) {
            Ok(ix) => {
                if ix.program_id == nara_quest::ID
                    && ix.data.len() >= 8
                    && ix.data[..8] == *nara_quest::client::args::SubmitAnswer::DISCRIMINATOR
                {
                    return Ok(true);
                }
                idx += 1;
            }
            Err(_) => break,
        }
    }
    Ok(false)
}
