use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint as MintInterface, TokenAccount as TokenAccountInterface};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::{AgentState, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::nara_quest;
use crate::seeds::*;
use super::helpers::{validate_referral_accounts, create_ata_and_mint};

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
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    #[account(
        mut,
        seeds = [SEED_POINT_MINT],
        bump,
    )]
    pub point_mint: InterfaceAccount<'info, MintInterface>,
    /// CHECK: Mint authority PDA for signing mint_to.
    #[account(
        seeds = [SEED_MINT_AUTHORITY],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = point_mint,
        associated_token::authority = authority,
        associated_token::token_program = token_program,
    )]
    pub authority_point_account: InterfaceAccount<'info, TokenAccountInterface>,
    /// Optional referral agent PDA. Pass null if no referral.
    pub referral_agent: Option<AccountLoader<'info, AgentState>>,
    /// CHECK: Optional referral authority wallet. Required when referral_agent is provided.
    #[account(mut)]
    pub referral_authority: Option<UncheckedAccount<'info>>,
    /// CHECK: Optional referral authority's ATA for point mint. Required when referral_agent is provided.
    #[account(mut)]
    pub referral_point_account: Option<UncheckedAccount<'info>>,
    /// CHECK: Referee Activity mint PDA.
    #[account(mut, seeds = [SEED_REFEREE_ACTIVITY_MINT], bump)]
    pub referee_activity_mint: UncheckedAccount<'info>,
    /// CHECK: Optional referral authority's ATA for referee activity mint.
    #[account(mut)]
    pub referral_referee_activity_account: Option<UncheckedAccount<'info>>,
    /// CHECK: Instructions sysvar for verifying submit_answer ix in tx.
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn log_activity(
    ctx: Context<LogActivity>,
    agent_id: String,
    model: String,
    activity: String,
    log: String,
) -> Result<()> {
    let clock = Clock::get()?;
    let instructions_account = ctx.accounts.instructions.to_account_info();

    // === Security: ban CPI calls ===
    let current_idx = ix_sysvar::load_current_index_checked(&instructions_account)?;
    let current_ix = ix_sysvar::load_instruction_at_checked(current_idx as usize, &instructions_account)?;
    require!(current_ix.program_id == crate::ID, AgentRegistryError::CpiNotAllowed);

    // === Security: single scan — count log_activity + find quest ===
    let self_disc = &current_ix.data[..8];
    let (log_activity_count, has_quest_ix) = scan_transaction_instructions(
        &instructions_account,
        &ctx.accounts.authority.key(),
        self_disc,
    )?;
    require!(log_activity_count <= 1, AgentRegistryError::DuplicateLogActivity);

    // === Security: validate referral matches agent's stored referral_id ===
    let has_valid_referral;
    let referral_id;
    {
        let agent = ctx.accounts.agent.load()?;
        if agent.has_referral() {
            if let Some(ref referral_loader) = ctx.accounts.referral_agent {
                // Verify the passed referral_agent matches the stored referral_id
                let expected_referral_pda = Pubkey::find_program_address(
                    &[SEED_AGENT, agent.referral_id_str().as_bytes()],
                    ctx.program_id,
                ).0;
                require_keys_eq!(
                    referral_loader.key(),
                    expected_referral_pda,
                    AgentRegistryError::ReferralNotFound
                );

                let referral_auth = ctx.accounts.referral_authority.as_ref()
                    .ok_or(AgentRegistryError::ReferralNotFound)?;
                validate_referral_accounts(
                    referral_loader,
                    &referral_auth.to_account_info(),
                    ctx.accounts.referral_point_account.as_ref().map(|a| a.as_ref()),
                    &ctx.accounts.point_mint.key(),
                )?;

                referral_id = agent.referral_id_str().to_string();
                has_valid_referral = true;
            } else {
                referral_id = String::new();
                has_valid_referral = false;
            }
        } else {
            referral_id = String::new();
            has_valid_referral = false;
        }
    }

    let mut points_earned: u64 = 0;
    let mut referral_points_earned: u64 = 0;

    if has_quest_ix {
        let config = ctx.accounts.config.load()?;
        let ps = config.points_self;
        let pr = config.points_referral;
        let mint_key = config.point_mint;
        let referee_activity_mint_key = config.referee_activity_mint;
        drop(config);

        let authority_bump = ctx.bumps.mint_authority;
        let authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[authority_bump]]];

        // Mint points_self tokens to authority
        if ps > 0 {
            invoke_signed(
                &spl_token_2022::instruction::mint_to(
                    &spl_token_2022::ID,
                    &mint_key,
                    &ctx.accounts.authority_point_account.key(),
                    &ctx.accounts.mint_authority.key(),
                    &[],
                    ps,
                )?,
                &[
                    ctx.accounts.point_mint.to_account_info(),
                    ctx.accounts.authority_point_account.to_account_info(),
                    ctx.accounts.mint_authority.to_account_info(),
                ],
                authority_seeds,
            )?;
            points_earned = ps;
        }

        // Mint points_referral tokens to referral authority
        if has_valid_referral && pr > 0 {
            if let (Some(ref referral_auth), Some(ref referral_point_acc)) = (
                &ctx.accounts.referral_authority,
                &ctx.accounts.referral_point_account,
            ) {
                create_ata_and_mint(
                    &ctx.accounts.authority.to_account_info(),
                    &referral_auth.to_account_info(),
                    &referral_point_acc.to_account_info(),
                    &ctx.accounts.point_mint.to_account_info(),
                    &ctx.accounts.mint_authority.to_account_info(),
                    authority_seeds,
                    &mint_key,
                    pr,
                    &ctx.accounts.system_program.to_account_info(),
                    &ctx.accounts.token_program.to_account_info(),
                    &ctx.accounts.associated_token_program.to_account_info(),
                )?;
                referral_points_earned = pr;

                // Mint 1 NARA Referee Activity token
                if let Some(ref referee_activity_acc) = ctx.accounts.referral_referee_activity_account {
                    create_ata_and_mint(
                        &ctx.accounts.authority.to_account_info(),
                        &referral_auth.to_account_info(),
                        &referee_activity_acc.to_account_info(),
                        &ctx.accounts.referee_activity_mint.to_account_info(),
                        &ctx.accounts.mint_authority.to_account_info(),
                        authority_seeds,
                        &referee_activity_mint_key,
                        1,
                        &ctx.accounts.system_program.to_account_info(),
                        &ctx.accounts.token_program.to_account_info(),
                        &ctx.accounts.associated_token_program.to_account_info(),
                    )?;
                }
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

/// Single scan over transaction instructions:
/// - Count how many log_activity instructions exist (by matching self_discriminator)
/// - Find a valid submit_answer instruction and verify user == authority
fn scan_transaction_instructions(
    instructions_account: &AccountInfo,
    authority: &Pubkey,
    self_discriminator: &[u8],
) -> Result<(u32, bool)> {
    let mut log_activity_count = 0u32;
    let mut has_valid_quest = false;
    let mut idx = 0u16;

    loop {
        match ix_sysvar::load_instruction_at_checked(idx as usize, instructions_account) {
            Ok(ix) => {
                // Count log_activity instructions
                if ix.program_id == crate::ID
                    && ix.data.len() >= 8
                    && ix.data[..8] == self_discriminator[..8]
                {
                    log_activity_count += 1;
                }

                // Find valid quest instruction
                if !has_valid_quest
                    && ix.program_id == nara_quest::ID
                    && ix.data.len() >= 8
                    && ix.data[..8] == *nara_quest::client::args::SubmitAnswer::DISCRIMINATOR
                {
                    // SubmitAnswer accounts per IDL: pool(0), winner_record(1), vault(2), user(3), payer(4), system_program(5)
                    require!(ix.accounts.len() > 3, AgentRegistryError::QuestIxNotFound);
                    require_keys_eq!(
                        ix.accounts[3].pubkey,
                        *authority,
                        AgentRegistryError::QuestUserMismatch
                    );
                    has_valid_quest = true;
                }

                idx += 1;
            }
            Err(_) => break,
        }
    }

    Ok((log_activity_count, has_valid_quest))
}
