use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct UpdateTwitterVerifier<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
}

pub fn update_twitter_verifier(ctx: Context<UpdateTwitterVerifier>, new_verifier: Pubkey) -> Result<()> {
    ctx.accounts.config.load_mut()?.twitter_verifier = new_verifier;
    Ok(())
}
