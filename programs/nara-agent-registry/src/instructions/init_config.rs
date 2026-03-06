use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::constants::{DEFAULT_REGISTER_FEE, DEFAULT_POINTS_SELF, DEFAULT_POINTS_REFERRAL};

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<ProgramConfig>(),
        seeds = [b"config"],
        bump,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    pub system_program: Program<'info, System>,
}

pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
    let mut config = ctx.accounts.config.load_init()?;
    config.admin = ctx.accounts.admin.key();
    config.register_fee = DEFAULT_REGISTER_FEE;
    config.fee_recipient = ctx.accounts.admin.key();
    config.points_self = DEFAULT_POINTS_SELF;
    config.points_referral = DEFAULT_POINTS_REFERRAL;
    Ok(())
}
