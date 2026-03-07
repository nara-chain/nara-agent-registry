use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::associated_token::AssociatedToken;
use crate::state::{AgentState, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::constants::{MIN_AGENT_ID_LEN, MAX_AGENT_ID_LEN};
use crate::seeds::*;
use super::helpers::{validate_referral_accounts, create_and_mint_referral_points};

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RegisterAgent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<AgentState>(),
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        seeds = [SEED_CONFIG],
        bump,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: must equal config.fee_recipient; validated in handler.
    #[account(mut)]
    pub fee_recipient: UncheckedAccount<'info>,
    /// CHECK: Point mint PDA.
    #[account(
        mut,
        seeds = [SEED_POINT_MINT],
        bump,
    )]
    pub point_mint: UncheckedAccount<'info>,
    /// CHECK: Mint authority PDA for signing mint_to.
    #[account(
        seeds = [SEED_MINT_AUTHORITY],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    /// Optional referral agent PDA. Pass null if no referral.
    pub referral_agent: Option<AccountLoader<'info, AgentState>>,
    /// CHECK: Optional referral authority to receive fee share. Required when referral_agent is provided.
    #[account(mut)]
    pub referral_authority: Option<UncheckedAccount<'info>>,
    /// CHECK: Optional referral authority's ATA for point mint. Required when referral_agent is provided.
    #[account(mut)]
    pub referral_point_account: Option<UncheckedAccount<'info>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
    require!(agent_id.len() >= MIN_AGENT_ID_LEN, AgentRegistryError::AgentIdTooShort);
    require!(agent_id.len() <= MAX_AGENT_ID_LEN, AgentRegistryError::AgentIdTooLong);
    require!(
        agent_id.chars().all(|c| !c.is_uppercase()),
        AgentRegistryError::AgentIdNotLowercase
    );

    let config = ctx.accounts.config.load()?;
    require_keys_eq!(
        ctx.accounts.fee_recipient.key(),
        config.fee_recipient,
        AgentRegistryError::InvalidFeeRecipient
    );

    let has_referral = ctx.accounts.referral_agent.is_some();

    if has_referral {
        let referral_loader = ctx.accounts.referral_agent.as_ref().unwrap();
        let referral_auth_account = ctx.accounts.referral_authority.as_ref()
            .ok_or(AgentRegistryError::ReferralNotFound)?;

        validate_referral_accounts(
            referral_loader,
            &referral_auth_account.to_account_info(),
            ctx.accounts.referral_point_account.as_ref().map(|a| a.as_ref()),
            &config.point_mint,
        )?;

        let fee = config.referral_register_fee;
        let referral_share = config.referral_fee_share;
        let system_share = fee.saturating_sub(referral_share);
        let referral_points = config.referral_register_points;
        let mint_key = config.point_mint;
        drop(config);

        // Transfer system's share to fee_recipient
        if system_share > 0 && ctx.accounts.fee_recipient.key() != ctx.accounts.authority.key() {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.authority.to_account_info(),
                        to: ctx.accounts.fee_recipient.to_account_info(),
                    },
                ),
                system_share,
            )?;
        }

        // Transfer referral's share to referral authority
        if referral_share > 0 {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.authority.to_account_info(),
                        to: referral_auth_account.to_account_info(),
                    },
                ),
                referral_share,
            )?;
        }

        // Mint referral points as tokens
        if referral_points > 0 {
            let referral_point_acc = ctx.accounts.referral_point_account.as_ref()
                .ok_or(AgentRegistryError::ReferralNotFound)?;

            let authority_bump = ctx.bumps.mint_authority;
            let authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[authority_bump]]];

            create_and_mint_referral_points(
                &ctx.accounts.authority.to_account_info(),
                &referral_auth_account.to_account_info(),
                &referral_point_acc.to_account_info(),
                &ctx.accounts.point_mint.to_account_info(),
                &ctx.accounts.mint_authority.to_account_info(),
                authority_seeds,
                &mint_key,
                referral_points,
                &ctx.accounts.system_program.to_account_info(),
                &ctx.accounts.token_program.to_account_info(),
                &ctx.accounts.associated_token_program.to_account_info(),
            )?;
        }
    } else {
        let fee = config.register_fee;
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
    }

    let now = Clock::get()?.unix_timestamp;
    let mut agent = ctx.accounts.agent.load_init()?;
    agent.authority = ctx.accounts.authority.key();
    agent.agent_id_len = agent_id.len() as u32;
    agent.agent_id[..agent_id.len()].copy_from_slice(agent_id.as_bytes());
    agent.pending_buffer = Pubkey::default();
    agent.memory = Pubkey::default();
    agent.version = 0;
    agent.created_at = now;
    agent.updated_at = 0;

    // Save referral agent_id if present
    if let Some(ref referral_loader) = ctx.accounts.referral_agent {
        let referral_record = referral_loader.load()?;
        let rid_len = referral_record.agent_id_len as usize;
        agent.referral_id_len = rid_len as u32;
        agent.referral_id[..rid_len].copy_from_slice(&referral_record.agent_id[..rid_len]);
    }

    Ok(())
}
