use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint as MintInterface, TokenAccount as TokenAccountInterface};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::{AgentState, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::constants::{MIN_AGENT_ID_LEN, MAX_AGENT_ID_LEN};
use crate::seeds::*;

// ── Direct registration (no referral) ────────────────────────────────────

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
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Fee vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
    pub fee_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
    validate_agent_id(&agent_id)?;

    let config = ctx.accounts.config.load()?;
    let fee = config.register_fee;
    drop(config);

    if fee > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.fee_vault.to_account_info(),
                },
            ),
            fee,
        )?;
    }

    init_agent_state(&ctx.accounts.agent, &ctx.accounts.authority.key(), &agent_id, None)?;

    Ok(())
}

// ── Registration with referral ───────────────────────────────────────────

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct RegisterAgentWithReferral<'info> {
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
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Fee vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
    pub fee_vault: UncheckedAccount<'info>,
    #[account(mut, seeds = [SEED_POINT_MINT], bump)]
    pub point_mint: InterfaceAccount<'info, MintInterface>,
    /// CHECK: Mint authority PDA for signing mint_to.
    #[account(seeds = [SEED_MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub referral_agent: AccountLoader<'info, AgentState>,
    /// CHECK: Referral authority; validated in handler against referral_agent.authority.
    #[account(mut)]
    pub referral_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = point_mint,
        associated_token::authority = referral_authority,
        associated_token::token_program = token_program,
    )]
    pub referral_point_account: InterfaceAccount<'info, TokenAccountInterface>,
    #[account(mut, seeds = [SEED_REFEREE_MINT], bump)]
    pub referee_mint: InterfaceAccount<'info, MintInterface>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = referee_mint,
        associated_token::authority = referral_authority,
        associated_token::token_program = token_program,
    )]
    pub referral_referee_account: InterfaceAccount<'info, TokenAccountInterface>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn register_agent_with_referral(ctx: Context<RegisterAgentWithReferral>, agent_id: String) -> Result<()> {
    validate_agent_id(&agent_id)?;

    let config = ctx.accounts.config.load()?;
    let fee = config.referral_register_fee;
    let referral_share = config.referral_fee_share;
    let system_share = fee.saturating_sub(referral_share);
    let referral_points = config.referral_register_points;
    drop(config);

    // Validate referral authority matches referral_agent.authority
    let referral_record = ctx.accounts.referral_agent.load()?;
    require_keys_eq!(
        ctx.accounts.referral_authority.key(),
        referral_record.authority,
        AgentRegistryError::InvalidReferralAuthority
    );
    let rid_len = referral_record.agent_id_len as usize;
    let mut rid = [0u8; 32];
    rid[..rid_len].copy_from_slice(&referral_record.agent_id[..rid_len]);
    drop(referral_record);

    // Transfer system's share to fee_vault
    if system_share > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.fee_vault.to_account_info(),
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
                    to: ctx.accounts.referral_authority.to_account_info(),
                },
            ),
            referral_share,
        )?;
    }

    let authority_bump = ctx.bumps.mint_authority;
    let authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[authority_bump]]];

    // Mint referral points
    if referral_points > 0 {
        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                &spl_token_2022::ID,
                &ctx.accounts.point_mint.key(),
                &ctx.accounts.referral_point_account.key(),
                &ctx.accounts.mint_authority.key(),
                &[],
                referral_points,
            )?,
            &[
                ctx.accounts.point_mint.to_account_info(),
                ctx.accounts.referral_point_account.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
            ],
            authority_seeds,
        )?;
    }

    // Mint 1 NARA Referee token
    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            &ctx.accounts.referee_mint.key(),
            &ctx.accounts.referral_referee_account.key(),
            &ctx.accounts.mint_authority.key(),
            &[],
            1,
        )?,
        &[
            ctx.accounts.referee_mint.to_account_info(),
            ctx.accounts.referral_referee_account.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
        ],
        authority_seeds,
    )?;

    init_agent_state(
        &ctx.accounts.agent,
        &ctx.accounts.authority.key(),
        &agent_id,
        Some((rid_len as u32, rid)),
    )?;

    Ok(())
}

// ── Shared helpers ───────────────────────────────────────────────────────

fn validate_agent_id(agent_id: &str) -> Result<()> {
    require!(agent_id.len() >= MIN_AGENT_ID_LEN, AgentRegistryError::AgentIdTooShort);
    require!(agent_id.len() <= MAX_AGENT_ID_LEN, AgentRegistryError::AgentIdTooLong);
    require!(
        agent_id.chars().all(|c| !c.is_uppercase()),
        AgentRegistryError::AgentIdNotLowercase
    );
    Ok(())
}

fn init_agent_state(
    agent_loader: &AccountLoader<AgentState>,
    authority: &Pubkey,
    agent_id: &str,
    referral: Option<(u32, [u8; 32])>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mut agent = agent_loader.load_init()?;
    agent.authority = *authority;
    agent.agent_id_len = agent_id.len() as u32;
    agent.agent_id[..agent_id.len()].copy_from_slice(agent_id.as_bytes());
    agent.pending_buffer = Pubkey::default();
    agent.memory = Pubkey::default();
    agent.version = 0;
    agent.created_at = now;
    agent.updated_at = 0;

    if let Some((rid_len, rid)) = referral {
        agent.referral_id_len = rid_len;
        agent.referral_id[..rid_len as usize].copy_from_slice(&rid[..rid_len as usize]);
    }

    Ok(())
}
