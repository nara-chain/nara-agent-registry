use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::Token2022;
use spl_token_2022::{
    extension::ExtensionType,
    instruction as token_instruction,
    state::Mint as MintState,
};
use spl_token_metadata_interface::state::TokenMetadata;
use crate::state::ProgramConfig;
use crate::constants::*;
use crate::seeds::*;

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
    /// CHECK: Point mint PDA, created in handler via CPI.
    #[account(
        mut,
        seeds = [SEED_POINT_MINT],
        bump,
    )]
    pub point_mint: UncheckedAccount<'info>,
    /// CHECK: Mint authority PDA, used as mint authority for point_mint.
    #[account(
        seeds = [SEED_MINT_AUTHORITY],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
    // --- Initialize config ---
    let mut config = ctx.accounts.config.load_init()?;
    config.admin = ctx.accounts.admin.key();
    config.register_fee = DEFAULT_REGISTER_FEE;
    config.fee_recipient = ctx.accounts.admin.key();
    config.point_mint = ctx.accounts.point_mint.key();
    config.points_self = DEFAULT_POINTS_SELF;
    config.points_referral = DEFAULT_POINTS_REFERRAL;
    config.referral_register_fee = DEFAULT_REFERRAL_REGISTER_FEE;
    config.referral_fee_share = DEFAULT_REFERRAL_FEE_SHARE;
    config.referral_register_points = DEFAULT_REFERRAL_REGISTER_POINTS;
    drop(config);

    // --- Create Token2022 mint with NonTransferable + MetadataPointer extensions ---
    let mint_bump = ctx.bumps.point_mint;
    let mint_signer_seeds: &[&[&[u8]]] = &[&[SEED_POINT_MINT, &[mint_bump]]];
    let mint_authority_key = ctx.accounts.mint_authority.key();
    let config_key = ctx.accounts.config.key();
    let mint_key = ctx.accounts.point_mint.key();

    // Step 1: Calculate space for fixed extensions only (no metadata yet)
    let extension_types = vec![
        ExtensionType::NonTransferable,
        ExtensionType::MetadataPointer,
    ];
    let mint_size = ExtensionType::try_calculate_account_len::<MintState>(&extension_types)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_size);

    // Step 2: Create mint account with fixed-extension space
    invoke_signed(
        &anchor_lang::solana_program::system_instruction::create_account(
            &ctx.accounts.admin.key(),
            &mint_key,
            lamports,
            mint_size as u64,
            &spl_token_2022::ID,
        ),
        &[
            ctx.accounts.admin.to_account_info(),
            ctx.accounts.point_mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        mint_signer_seeds,
    )?;

    // Step 3: Initialize extensions (must be before InitializeMint2)
    invoke_signed(
        &token_instruction::initialize_non_transferable_mint(
            &spl_token_2022::ID,
            &mint_key,
        )?,
        &[ctx.accounts.point_mint.to_account_info()],
        mint_signer_seeds,
    )?;

    invoke_signed(
        &spl_token_2022::extension::metadata_pointer::instruction::initialize(
            &spl_token_2022::ID,
            &mint_key,
            Some(config_key),
            Some(mint_key),
        )?,
        &[ctx.accounts.point_mint.to_account_info()],
        mint_signer_seeds,
    )?;

    // Step 4: Initialize Mint (decimals=0, mint_authority=mint_authority PDA)
    invoke_signed(
        &token_instruction::initialize_mint2(
            &spl_token_2022::ID,
            &mint_key,
            &mint_authority_key, // mint authority = mint_authority PDA
            Some(&config_key),   // freeze authority = config PDA
            POINT_TOKEN_DECIMALS,
        )?,
        &[ctx.accounts.point_mint.to_account_info()],
        mint_signer_seeds,
    )?;

    // Step 5: Initialize TokenMetadata (must be after InitializeMint2)
    // Token2022 handles realloc internally; we just need to provide enough lamports.
    let meta = TokenMetadata {
        name: POINT_TOKEN_NAME.to_string(),
        symbol: POINT_TOKEN_SYMBOL.to_string(),
        uri: POINT_TOKEN_URI.to_string(),
        update_authority: Some(config_key).try_into().unwrap(),
        mint: mint_key,
        ..Default::default()
    };
    let meta_len = meta.tlv_size_of().map_err(|_| ProgramError::InvalidAccountData)?;
    let new_size = mint_size + meta_len;
    let new_lamports = rent.minimum_balance(new_size);
    let extra_lamports = new_lamports.saturating_sub(lamports);

    // Transfer extra rent for metadata space
    if extra_lamports > 0 {
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.admin.key(),
                &mint_key,
                extra_lamports,
            ),
            &[
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.point_mint.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
    }

    // TokenMetadata::initialize requires mint_authority to sign
    invoke_signed(
        &spl_token_metadata_interface::instruction::initialize(
            &spl_token_2022::ID,
            &mint_key,
            &config_key,          // update authority
            &mint_key,
            &mint_authority_key,  // mint authority signs
            meta.name,
            meta.symbol,
            meta.uri,
        ),
        &[
            ctx.accounts.point_mint.to_account_info(),
            ctx.accounts.config.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
        ],
        &[&[SEED_MINT_AUTHORITY, &[ctx.bumps.mint_authority]]],
    )?;

    Ok(())
}
