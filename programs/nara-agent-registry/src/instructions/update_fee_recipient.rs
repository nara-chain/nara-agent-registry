use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;

#[derive(Accounts)]
pub struct UpdateFeeRecipient<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"config"],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: Account<'info, ProgramConfig>,
}

pub fn update_fee_recipient(ctx: Context<UpdateFeeRecipient>, new_recipient: Pubkey) -> Result<()> {
    ctx.accounts.config.fee_recipient = new_recipient;
    Ok(())
}
