use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct WithdrawTwitterVerifyFees<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Twitter verify vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_TWITTER_VERIFY_VAULT], bump)]
    pub twitter_verify_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw_twitter_verify_fees(ctx: Context<WithdrawTwitterVerifyFees>, amount: u64) -> Result<()> {
    let vault_balance = ctx.accounts.twitter_verify_vault.lamports();
    let rent_exempt = Rent::get()?.minimum_balance(0);
    let available = vault_balance.saturating_sub(rent_exempt);

    require!(
        available >= amount,
        AgentRegistryError::InsufficientTwitterVerifyVaultBalance
    );

    let vault_bump = ctx.bumps.twitter_verify_vault;
    let signer_seeds: &[&[&[u8]]] = &[&[SEED_TWITTER_VERIFY_VAULT, &[vault_bump]]];

    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.twitter_verify_vault.to_account_info(),
                to: ctx.accounts.admin.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    Ok(())
}
