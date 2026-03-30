use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;
#[derive(Accounts)]
pub struct UpdateRegisterFee<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_register_fee(ctx: Context<UpdateRegisterFee>, fee: u64, fee_7: u64, fee_6: u64, fee_5: u64) -> Result<()> {
    let mut config = ctx.accounts.config.load_mut()?;
    config.register_fee = fee;
    config.register_fee_7 = fee_7;
    config.register_fee_6 = fee_6;
    config.register_fee_5 = fee_5;
    Ok(())
}
