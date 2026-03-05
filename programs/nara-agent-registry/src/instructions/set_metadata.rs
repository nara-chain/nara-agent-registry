use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::state::{AgentRecord, AgentMetadata};
use crate::error::AgentRegistryError;

#[derive(Accounts)]
#[instruction(agent_id: String, data: String)]
pub struct SetMetadata<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"agent", agent_id.as_bytes()],
        bump,
        has_one = authority @ AgentRegistryError::Unauthorized,
    )]
    pub agent: AccountLoader<'info, AgentRecord>,
    /// CHECK: AgentMetadata PDA — created or resized in the handler.
    #[account(
        mut,
        seeds = [b"meta", agent.key().as_ref()],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn set_metadata(ctx: Context<SetMetadata>, _agent_id: String, data: String) -> Result<()> {
    let needed = AgentMetadata::space(data.len());
    let info = ctx.accounts.metadata.to_account_info();
    let len_offset = AgentMetadata::HEADER_SIZE;
    let data_offset = len_offset + 4;

    if info.lamports() == 0 {
        let agent_key = ctx.accounts.agent.key();
        let seeds: &[&[u8]] = &[b"meta", agent_key.as_ref()];
        let (_, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        let signer_seeds: &[&[&[u8]]] = &[&[b"meta", agent_key.as_ref(), &[bump]]];

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

        let mut account_data = info.try_borrow_mut_data()?;
        account_data[..8].copy_from_slice(&AgentMetadata::DISCRIMINATOR);
        let data_bytes = data.as_bytes();
        account_data[len_offset..len_offset + 4].copy_from_slice(&(data_bytes.len() as u32).to_le_bytes());
        account_data[data_offset..data_offset + data_bytes.len()].copy_from_slice(data_bytes);
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

        let mut account_data = info.try_borrow_mut_data()?;
        let data_bytes = data.as_bytes();
        account_data[len_offset..len_offset + 4].copy_from_slice(&(data_bytes.len() as u32).to_le_bytes());
        account_data[data_offset..data_offset + data_bytes.len()].copy_from_slice(data_bytes);
    }

    Ok(())
}
