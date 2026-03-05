use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::DEFAULT_REGISTER_FEE;

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 8 + 32, // discriminator + admin + register_fee + fee_recipient
        seeds = [b"config"],
        bump,
    )]
    pub config: Account<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
}

/// One-time program initialization. The signer becomes the admin.
/// Sets register_fee = 1 SOL and fee_recipient = admin.
pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.register_fee = DEFAULT_REGISTER_FEE;
    config.fee_recipient = ctx.accounts.admin.key();
    Ok(())
}
