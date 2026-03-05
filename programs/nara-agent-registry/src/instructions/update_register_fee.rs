use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;

#[derive(Accounts)]
pub struct UpdateRegisterFee<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config"],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: Account<'info, ProgramConfig>,
}

pub fn update_register_fee(ctx: Context<UpdateRegisterFee>, new_fee: u64) -> Result<()> {
    ctx.accounts.config.register_fee = new_fee;
    Ok(())
}
