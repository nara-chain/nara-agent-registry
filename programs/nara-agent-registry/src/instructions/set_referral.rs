use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::associated_token::AssociatedToken;
use crate::state::{AgentState, ProgramConfig};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::helpers::create_ata_and_mint;

#[derive(Accounts)]
#[instruction(agent_id: String)]
pub struct SetReferral<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    pub referral_agent: AccountLoader<'info, AgentState>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Referee mint PDA.
    #[account(mut, seeds = [SEED_REFEREE_MINT], bump)]
    pub referee_mint: UncheckedAccount<'info>,
    /// CHECK: Mint authority PDA.
    #[account(seeds = [SEED_MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: Referral authority wallet to receive referee token.
    #[account(mut)]
    pub referral_authority: UncheckedAccount<'info>,
    /// CHECK: Referral authority's ATA for referee mint.
    #[account(mut)]
    pub referral_referee_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn set_referral(ctx: Context<SetReferral>, _agent_id: String) -> Result<()> {
    let mut agent = ctx.accounts.agent.load_mut()?;

    require!(agent.referral_id_len == 0, AgentRegistryError::ReferralAlreadySet);

    require!(
        ctx.accounts.referral_agent.key() != ctx.accounts.agent.key(),
        AgentRegistryError::SelfReferral
    );

    let referral = ctx.accounts.referral_agent.load()?;
    let rid_len = referral.agent_id_len as usize;
    agent.referral_id_len = rid_len as u32;
    agent.referral_id[..rid_len].copy_from_slice(&referral.agent_id[..rid_len]);

    require_keys_eq!(
        ctx.accounts.referral_authority.key(),
        referral.authority,
        AgentRegistryError::InvalidReferralAuthority
    );
    drop(referral);
    drop(agent);

    let config = ctx.accounts.config.load()?;
    let referee_mint_key = config.referee_mint;
    drop(config);

    let authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[ctx.bumps.mint_authority]]];

    create_ata_and_mint(
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.referral_authority.to_account_info(),
        &ctx.accounts.referral_referee_account.to_account_info(),
        &ctx.accounts.referee_mint.to_account_info(),
        &ctx.accounts.mint_authority.to_account_info(),
        authority_seeds,
        &referee_mint_key,
        1,
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.token_program.to_account_info(),
        &ctx.accounts.associated_token_program.to_account_info(),
    )?;

    Ok(())
}
