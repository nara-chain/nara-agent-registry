use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use crate::state::ProgramConfig;
use crate::constants::*;
use crate::seeds::*;
use super::helpers::create_token2022_mint;

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<ProgramConfig>(),
        seeds = [SEED_CONFIG],
        bump,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Fee vault PDA for holding registration fees; validated by seeds constraint.
    #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
    pub fee_vault: UncheckedAccount<'info>,
    /// CHECK: Point mint PDA, created in handler via CPI.
    #[account(mut, seeds = [SEED_POINT_MINT], bump)]
    pub point_mint: UncheckedAccount<'info>,
    /// CHECK: Referee mint PDA, created in handler via CPI.
    #[account(mut, seeds = [SEED_REFEREE_MINT], bump)]
    pub referee_mint: UncheckedAccount<'info>,
    /// CHECK: Referee Activity mint PDA, created in handler via CPI.
    #[account(mut, seeds = [SEED_REFEREE_ACTIVITY_MINT], bump)]
    pub referee_activity_mint: UncheckedAccount<'info>,
    /// CHECK: Mint authority PDA, used as mint authority for all mints.
    #[account(seeds = [SEED_MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
    let mut config = ctx.accounts.config.load_init()?;
    config.admin = ctx.accounts.admin.key();
    config.register_fee = DEFAULT_REGISTER_FEE;
    config.fee_vault = ctx.accounts.fee_vault.key();
    config.point_mint = ctx.accounts.point_mint.key();
    config.referee_mint = ctx.accounts.referee_mint.key();
    config.referee_activity_mint = ctx.accounts.referee_activity_mint.key();
    config.points_self = DEFAULT_POINTS_SELF;
    config.points_referral = DEFAULT_POINTS_REFERRAL;
    config.referral_register_fee = DEFAULT_REFERRAL_REGISTER_FEE;
    config.referral_fee_share = DEFAULT_REFERRAL_FEE_SHARE;
    config.referral_register_points = DEFAULT_REFERRAL_REGISTER_POINTS;
    config.activity_reward = DEFAULT_ACTIVITY_REWARD;
    config.referral_activity_reward = DEFAULT_REFERRAL_ACTIVITY_REWARD;
    drop(config);

    let mint_authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[ctx.bumps.mint_authority]]];

    create_token2022_mint(
        &ctx.accounts.admin.to_account_info(),
        &ctx.accounts.point_mint.to_account_info(),
        &[&[SEED_POINT_MINT, &[ctx.bumps.point_mint]]],
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_seeds,
        &ctx.accounts.config.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        POINT_TOKEN_NAME.to_string(),
        POINT_TOKEN_SYMBOL.to_string(),
        POINT_TOKEN_URI.to_string(),
    )?;

    create_token2022_mint(
        &ctx.accounts.admin.to_account_info(),
        &ctx.accounts.referee_mint.to_account_info(),
        &[&[SEED_REFEREE_MINT, &[ctx.bumps.referee_mint]]],
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_seeds,
        &ctx.accounts.config.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        REFEREE_TOKEN_NAME.to_string(),
        REFEREE_TOKEN_SYMBOL.to_string(),
        REFEREE_TOKEN_URI.to_string(),
    )?;

    create_token2022_mint(
        &ctx.accounts.admin.to_account_info(),
        &ctx.accounts.referee_activity_mint.to_account_info(),
        &[&[SEED_REFEREE_ACTIVITY_MINT, &[ctx.bumps.referee_activity_mint]]],
        &ctx.accounts.mint_authority.to_account_info(),
        mint_authority_seeds,
        &ctx.accounts.config.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        REFEREE_ACTIVITY_TOKEN_NAME.to_string(),
        REFEREE_ACTIVITY_TOKEN_SYMBOL.to_string(),
        REFEREE_ACTIVITY_TOKEN_URI.to_string(),
    )?;

    Ok(())
}
