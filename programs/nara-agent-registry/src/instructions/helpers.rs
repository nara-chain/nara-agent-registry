use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::system_program as sol_system;
use crate::state::AgentState;
use crate::error::AgentRegistryError;

/// Create or resize a dynamic PDA and write discriminator + len-prefixed data.
/// Used by set_bio and set_metadata.
pub fn write_dynamic_pda<'a>(
    account: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    pda_seeds: &[&[u8]],
    discriminator: &[u8],
    header_size: usize,
    data: &[u8],
    program_id: &Pubkey,
) -> Result<()> {
    let len_offset = header_size;
    let data_offset = len_offset + 4;
    let needed = header_size + 4 + data.len();

    if account.lamports() == 0 {
        let (_, bump) = Pubkey::find_program_address(pda_seeds, program_id);
        let mut seeds_with_bump: Vec<&[u8]> = pda_seeds.to_vec();
        let bump_bytes = [bump];
        seeds_with_bump.push(&bump_bytes);
        let signer_seeds: &[&[&[u8]]] = &[&seeds_with_bump];

        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(needed);

        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                system_program.clone(),
                anchor_lang::system_program::CreateAccount {
                    from: authority.clone(),
                    to: account.clone(),
                },
                signer_seeds,
            ),
            lamports,
            needed as u64,
            program_id,
        )?;

        let mut account_data = account.try_borrow_mut_data()?;
        account_data[..8].copy_from_slice(discriminator);
        account_data[len_offset..len_offset + 4].copy_from_slice(&(data.len() as u32).to_le_bytes());
        account_data[data_offset..data_offset + data.len()].copy_from_slice(data);
    } else {
        let current = account.data_len();
        if current != needed {
            account.resize(needed)?;

            let rent = Rent::get()?;
            let new_min = rent.minimum_balance(needed);
            let current_lamports = account.lamports();
            if new_min > current_lamports {
                let diff = new_min - current_lamports;
                anchor_lang::system_program::transfer(
                    CpiContext::new(
                        system_program.clone(),
                        anchor_lang::system_program::Transfer {
                            from: authority.clone(),
                            to: account.clone(),
                        },
                    ),
                    diff,
                )?;
            } else if current_lamports > new_min {
                let diff = current_lamports - new_min;
                **account.try_borrow_mut_lamports()? -= diff;
                **authority.try_borrow_mut_lamports()? += diff;
            }
        }

        let mut account_data = account.try_borrow_mut_data()?;
        account_data[len_offset..len_offset + 4].copy_from_slice(&(data.len() as u32).to_le_bytes());
        account_data[data_offset..data_offset + data.len()].copy_from_slice(data);
    }

    Ok(())
}

/// Validate referral authority and ATA address.
pub fn validate_referral_accounts(
    referral_agent: &AccountLoader<AgentState>,
    referral_authority: &AccountInfo,
    referral_point_account: Option<&AccountInfo>,
    point_mint: &Pubkey,
) -> Result<()> {
    let referral_record = referral_agent.load()?;
    require_keys_eq!(
        referral_authority.key(),
        referral_record.authority,
        AgentRegistryError::InvalidReferralAuthority
    );
    drop(referral_record);

    if let Some(referral_point_acc) = referral_point_account {
        let expected_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
            &referral_authority.key(),
            point_mint,
            &spl_token_2022::ID,
        );
        require_keys_eq!(
            referral_point_acc.key(),
            expected_ata,
            AgentRegistryError::InvalidReferralPointAccount
        );
    }

    Ok(())
}

/// Create referral authority's ATA if needed and mint points.
pub fn create_and_mint_referral_points<'a>(
    payer: &AccountInfo<'a>,
    referral_authority: &AccountInfo<'a>,
    referral_point_account: &AccountInfo<'a>,
    point_mint: &AccountInfo<'a>,
    mint_authority: &AccountInfo<'a>,
    mint_authority_seeds: &[&[&[u8]]],
    mint_key: &Pubkey,
    amount: u64,
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    associated_token_program: &AccountInfo<'a>,
) -> Result<()> {
    if referral_point_account.data_is_empty() {
        anchor_lang::solana_program::program::invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                payer.key,
                referral_authority.key,
                mint_key,
                &spl_token_2022::ID,
            ),
            &[
                payer.clone(),
                referral_point_account.clone(),
                referral_authority.clone(),
                point_mint.clone(),
                system_program.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    }

    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            mint_key,
            &referral_point_account.key(),
            &mint_authority.key(),
            &[],
            amount,
        )?,
        &[
            point_mint.clone(),
            referral_point_account.clone(),
            mint_authority.clone(),
        ],
        mint_authority_seeds,
    )?;

    Ok(())
}

/// Close a raw (non-Anchor-managed) account by zeroing data, transferring lamports,
/// and reassigning ownership to the system program.
pub fn close_raw_account(account: &AccountInfo, destination: &AccountInfo) -> Result<()> {
    let lamports = account.lamports();
    **account.try_borrow_mut_lamports()? = 0;
    **destination.try_borrow_mut_lamports()? += lamports;
    account.assign(&sol_system::ID);
    account.try_borrow_mut_data()?.fill(0);
    Ok(())
}
