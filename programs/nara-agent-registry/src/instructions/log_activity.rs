use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use crate::state::{AgentRecord, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::nara_quest;

#[event]
pub struct ActivityLogged {
    pub agent_id: String,
    pub authority: Pubkey,
    pub model: String,
    pub activity: String,
    pub log: String,
    pub referral_id: String,
    pub points_earned: u64,
    pub referral_points_earned: u64,
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
    #[account(seeds = [b"config"], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
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

    let self_referral = ctx.accounts.referral_agent.as_ref()
        .map(|r| r.key() == ctx.accounts.agent.key())
        .unwrap_or(false);

    let referral_id = if !self_referral {
        if let Some(ref referral_loader) = ctx.accounts.referral_agent {
            let r = referral_loader.load()?;
            r.agent_id_str().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut points_earned: u64 = 0;
    let mut referral_points_earned: u64 = 0;

    if has_quest_ix {
        let config = ctx.accounts.config.load()?;
        let ps = config.points_self;
        let pr = config.points_referral;
        drop(config);

        let mut agent = ctx.accounts.agent.load_mut()?;
        agent.points += ps;
        points_earned = ps;
        drop(agent);

        if !self_referral {
            if let Some(ref referral_loader) = ctx.accounts.referral_agent {
                let mut referral_record = referral_loader.load_mut()?;
                referral_record.points += pr;
                referral_points_earned = pr;
            }
        }
    }

    emit!(ActivityLogged {
        agent_id,
        authority: ctx.accounts.authority.key(),
        model,
        activity,
        log,
        referral_id,
        points_earned,
        referral_points_earned,
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
