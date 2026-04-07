use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{Mint as MintInterface, TokenAccount as TokenAccountInterface};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::{ProgramConfig, AgentState, AgentTwitter, TwitterHandle};
use crate::error::AgentRegistryError;
use crate::seeds::*;
use super::verify_twitter::TwitterBindResult;

#[derive(Accounts)]
#[instruction(agent_id: String, username: String)]
pub struct ApproveRejectedTwitter<'info> {
    #[account(mut)]
    pub verifier: Signer<'info>,
    #[account(seeds = [SEED_CONFIG], bump)]
    pub config: AccountLoader<'info, ProgramConfig>,
    #[account(
        seeds = [SEED_AGENT, agent_id.as_bytes()],
        bump,
    )]
    pub agent: AccountLoader<'info, AgentState>,
    #[account(
        mut,
        seeds = [SEED_TWITTER, agent.key().as_ref()],
        bump,
    )]
    pub twitter: AccountLoader<'info, AgentTwitter>,
    #[account(
        init_if_needed,
        payer = verifier,
        space = 8 + std::mem::size_of::<TwitterHandle>(),
        seeds = [SEED_TWITTER_HANDLE, username.as_bytes()],
        bump,
    )]
    pub twitter_handle: AccountLoader<'info, TwitterHandle>,
    /// CHECK: Agent authority, receives fee refund. Validated in handler against agent.authority.
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: Twitter verify vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_TWITTER_VERIFY_VAULT], bump)]
    pub twitter_verify_vault: UncheckedAccount<'info>,
    /// CHECK: Treasury PDA for optional reward.
    #[account(mut, seeds = [SEED_TREASURY], bump)]
    pub treasury: UncheckedAccount<'info>,
    #[account(mut, seeds = [SEED_POINT_MINT], bump)]
    pub point_mint: InterfaceAccount<'info, MintInterface>,
    /// CHECK: Mint authority PDA for signing mint_to.
    #[account(seeds = [SEED_MINT_AUTHORITY], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = verifier,
        associated_token::mint = point_mint,
        associated_token::authority = authority,
        associated_token::token_program = token_program,
    )]
    pub authority_point_account: InterfaceAccount<'info, TokenAccountInterface>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn approve_rejected_twitter(ctx: Context<ApproveRejectedTwitter>, _agent_id: String, username: String) -> Result<()> {
    let config = ctx.accounts.config.load()?;
    require!(
        config.twitter_verifier != Pubkey::default(),
        AgentRegistryError::TwitterVerifierNotSet
    );
    require_keys_eq!(
        ctx.accounts.verifier.key(),
        config.twitter_verifier,
        AgentRegistryError::NotTwitterVerifier
    );

    let fee = config.twitter_verification_fee;
    let reward = config.twitter_verification_reward;
    let points = config.twitter_verification_points;
    drop(config);

    // Validate agent authority
    let agent = ctx.accounts.agent.load()?;
    require_keys_eq!(
        ctx.accounts.authority.key(),
        agent.authority,
        AgentRegistryError::Unauthorized
    );
    drop(agent);

    // Must be Rejected status
    let mut twitter = ctx.accounts.twitter.load_mut()?;
    require!(twitter.status == 3, AgentRegistryError::TwitterNotRejected);

    // Verify the username param matches stored username
    let stored_len = twitter.username_len as usize;
    let stored_username = &twitter.username[..stored_len];
    require!(
        username.as_bytes() == stored_username,
        AgentRegistryError::TwitterUsernameEmpty
    );

    // Set verified
    twitter.status = 2; // Verified
    twitter.verified_at = Clock::get()?.unix_timestamp;
    drop(twitter);

    // Init or reuse TwitterHandle. is_first_bind = brand new TwitterHandle PDA.
    let is_first_bind = {
        let acc_info = ctx.accounts.twitter_handle.to_account_info();
        let data = acc_info.try_borrow_data()?;
        data[..8] == [0u8; 8]
    };
    let mut handle = if is_first_bind {
        ctx.accounts.twitter_handle.load_init()?
    } else {
        let h = ctx.accounts.twitter_handle.load_mut()?;
        require_keys_eq!(
            h.agent,
            Pubkey::default(),
            AgentRegistryError::TwitterHandleAlreadyTaken
        );
        h
    };
    handle.agent = ctx.accounts.agent.key();
    drop(handle);

    // Skip rewards/points if this twitter has been rewarded before
    let reward = if is_first_bind { reward } else { 0 };
    let points = if is_first_bind { points } else { 0 };

    // Refund verification fee from twitter_verify_vault
    let vault_bump = ctx.bumps.twitter_verify_vault;
    let vault_seeds: &[&[&[u8]]] = &[&[SEED_TWITTER_VERIFY_VAULT, &[vault_bump]]];

    if fee > 0 {
        let vault_balance = ctx.accounts.twitter_verify_vault.lamports();
        let rent_exempt = Rent::get()?.minimum_balance(0);
        let available = vault_balance.saturating_sub(rent_exempt);

        if available >= fee {
            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.twitter_verify_vault.to_account_info(),
                        to: ctx.accounts.authority.to_account_info(),
                    },
                    vault_seeds,
                ),
                fee,
            )?;
        }
    }

    // Optional: transfer reward from treasury
    if reward > 0 {
        let treasury_bump = ctx.bumps.treasury;
        let treasury_seeds: &[&[&[u8]]] = &[&[SEED_TREASURY, &[treasury_bump]]];
        let treasury_balance = ctx.accounts.treasury.lamports();
        let rent_exempt = Rent::get()?.minimum_balance(0);
        let available = treasury_balance.saturating_sub(rent_exempt);

        if available >= reward {
            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.treasury.to_account_info(),
                        to: ctx.accounts.authority.to_account_info(),
                    },
                    treasury_seeds,
                ),
                reward,
            )?;
        }
    }

    // Optional: mint points
    if points > 0 {
        let authority_bump = ctx.bumps.mint_authority;
        let authority_seeds: &[&[&[u8]]] = &[&[SEED_MINT_AUTHORITY, &[authority_bump]]];

        invoke_signed(
            &spl_token_2022::instruction::mint_to(
                &spl_token_2022::ID,
                &ctx.accounts.point_mint.key(),
                &ctx.accounts.authority_point_account.key(),
                &ctx.accounts.mint_authority.key(),
                &[],
                points,
            )?,
            &[
                ctx.accounts.point_mint.to_account_info(),
                ctx.accounts.authority_point_account.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
            ],
            authority_seeds,
        )?;
    }

    msg!("approve_rejected_twitter: agent={}, username={}, fee_refunded={}, reward={}, points={}", _agent_id, username, fee, reward, points);

    emit!(TwitterBindResult {
        agent_id: _agent_id,
        authority: ctx.accounts.authority.key(),
        username,
        approved: true,
        fee_refunded: fee,
        reward,
        points,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
