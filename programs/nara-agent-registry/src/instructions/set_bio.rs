use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::state::{AgentRecord, AgentBio};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String, bio: String)]
pub struct SetBio<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    /// CHECK: AgentBio PDA — created or resized in the handler.
    #[account(
        mut,
        seeds = [b"bio", agent.key().as_ref()],
        bump,
    )]
    pub bio_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn set_bio(
    ctx: Context<SetBio>,
    _agent_id: String,
    bio: String,
) -> Result<()> {
    let needed = AgentBio::space(bio.len());
    let info = ctx.accounts.bio_account.to_account_info();
    let len_offset = AgentBio::HEADER_SIZE;
    let data_offset = len_offset + 4;

    if info.lamports() == 0 {
        // First time — create the PDA.
        let agent_key = ctx.accounts.agent.key();
        let seeds: &[&[u8]] = &[b"bio", agent_key.as_ref()];
        let (_, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        let signer_seeds: &[&[&[u8]]] = &[&[b"bio", agent_key.as_ref(), &[bump]]];

        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(needed);

        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: ctx.accounts.authority.to_account_info(),
                    to: info.clone(),
                },
                signer_seeds,
            ),
            lamports,
            needed as u64,
            ctx.program_id,
        )?;

        let mut data = info.try_borrow_mut_data()?;
        data[..8].copy_from_slice(&AgentBio::DISCRIMINATOR);
        let bio_bytes = bio.as_bytes();
        data[len_offset..len_offset + 4].copy_from_slice(&(bio_bytes.len() as u32).to_le_bytes());
        data[data_offset..data_offset + bio_bytes.len()].copy_from_slice(bio_bytes);
    } else {
        let current = info.data_len();
        if current != needed {
            info.resize(needed)?;

            let rent = Rent::get()?;
            let new_min = rent.minimum_balance(needed);
            let current_lamports = info.lamports();
            if new_min > current_lamports {
                let diff = new_min - current_lamports;
                system_program::transfer(
                    CpiContext::new(
                        ctx.accounts.system_program.to_account_info(),
                        system_program::Transfer {
                            from: ctx.accounts.authority.to_account_info(),
                            to: info.clone(),
                        },
                    ),
                    diff,
                )?;
            } else if current_lamports > new_min {
                let diff = current_lamports - new_min;
                **info.try_borrow_mut_lamports()? -= diff;
                **ctx.accounts.authority.try_borrow_mut_lamports()? += diff;
            }
        }

        let mut data = info.try_borrow_mut_data()?;
        let bio_bytes = bio.as_bytes();
        data[len_offset..len_offset + 4].copy_from_slice(&(bio_bytes.len() as u32).to_le_bytes());
        data[data_offset..data_offset + bio_bytes.len()].copy_from_slice(bio_bytes);
    }

    Ok(())
}
