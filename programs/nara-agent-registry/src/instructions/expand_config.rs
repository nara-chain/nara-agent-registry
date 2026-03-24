use anchor_lang::prelude::*;
use crate::error::AgentRegistryError;
use crate::seeds::SEED_CONFIG;

#[derive(Accounts)]
pub struct ExpandConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Config PDA resized manually to avoid Anchor's fixed-space constraint.
    #[account(
        mut,
        seeds = [SEED_CONFIG],
        bump,
    )]
    pub config: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Grow the config account by `extend_size` bytes.
/// Only callable by admin.
/// After calling this on-chain, update the ProgramConfig struct to use the new reserved bytes.
pub fn expand_config(ctx: Context<ExpandConfig>, extend_size: u64) -> Result<()> {
    let config_info = ctx.accounts.config.to_account_info();

    // Validate caller is admin by loading the existing config header.
    {
        let data = config_info.try_borrow_data()?;
        require!(data.len() >= 8 + 32, AgentRegistryError::Unauthorized);
        // admin pubkey is the first field after the 8-byte discriminator.
        let admin_bytes: [u8; 32] = data[8..40].try_into().unwrap();
        let stored_admin = Pubkey::from(admin_bytes);
        require_keys_eq!(stored_admin, ctx.accounts.admin.key(), AgentRegistryError::Unauthorized);
    }

    require!(extend_size > 0, AgentRegistryError::Unauthorized);

    let target_size = config_info.data_len() as u64 + extend_size;
    config_info.resize(target_size as usize)?;

    // Fund additional rent if needed.
    let rent = Rent::get()?;
    let needed = rent.minimum_balance(target_size as usize);
    let current_lamports = config_info.lamports();
    if needed > current_lamports {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.admin.to_account_info(),
                    to: config_info.clone(),
                },
            ),
            needed - current_lamports,
        )?;
    }

    Ok(())
}
