use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::system_program as sol_system;
use crate::state::TwitterQueue;

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

/// Create referral authority's ATA if needed and mint points.
pub fn create_ata_and_mint<'a>(
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

/// Create a Token2022 mint with NonTransferable + MetadataPointer extensions.
pub fn create_token2022_mint<'a>(
    payer: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    mint_signer_seeds: &[&[&[u8]]],
    mint_authority: &AccountInfo<'a>,
    mint_authority_seeds: &[&[&[u8]]],
    config: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    use spl_token_2022::{
        extension::ExtensionType,
        instruction as token_instruction,
        state::Mint as MintState,
    };
    use spl_token_metadata_interface::state::TokenMetadata;
    use anchor_lang::solana_program::program::invoke_signed;

    let mint_authority_key = mint_authority.key();
    let config_key = config.key();
    let mint_key = mint.key();

    let extension_types = vec![
        ExtensionType::NonTransferable,
        ExtensionType::MetadataPointer,
    ];
    let mint_size = ExtensionType::try_calculate_account_len::<MintState>(&extension_types)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(mint_size);

    invoke_signed(
        &anchor_lang::solana_program::system_instruction::create_account(
            payer.key,
            &mint_key,
            lamports,
            mint_size as u64,
            &spl_token_2022::ID,
        ),
        &[payer.clone(), mint.clone(), system_program.clone()],
        mint_signer_seeds,
    )?;

    invoke_signed(
        &token_instruction::initialize_non_transferable_mint(&spl_token_2022::ID, &mint_key)?,
        &[mint.clone()],
        mint_signer_seeds,
    )?;

    invoke_signed(
        &spl_token_2022::extension::metadata_pointer::instruction::initialize(
            &spl_token_2022::ID,
            &mint_key,
            Some(config_key),
            Some(mint_key),
        )?,
        &[mint.clone()],
        mint_signer_seeds,
    )?;

    invoke_signed(
        &token_instruction::initialize_mint2(
            &spl_token_2022::ID,
            &mint_key,
            &mint_authority_key,
            Some(&config_key),
            0,
        )?,
        &[mint.clone()],
        mint_signer_seeds,
    )?;

    let meta = TokenMetadata {
        name,
        symbol,
        uri,
        update_authority: Some(config_key).try_into().unwrap(),
        mint: mint_key,
        ..Default::default()
    };
    let meta_len = meta.tlv_size_of().map_err(|_| ProgramError::InvalidAccountData)?;
    let new_size = mint_size + meta_len;
    let new_lamports = rent.minimum_balance(new_size);
    let extra_lamports = new_lamports.saturating_sub(lamports);

    if extra_lamports > 0 {
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                payer.key,
                &mint_key,
                extra_lamports,
            ),
            &[payer.clone(), mint.clone(), system_program.clone()],
        )?;
    }

    invoke_signed(
        &spl_token_metadata_interface::instruction::initialize(
            &spl_token_2022::ID,
            &mint_key,
            &config_key,
            &mint_key,
            &mint_authority_key,
            meta.name,
            meta.symbol,
            meta.uri,
        ),
        &[mint.clone(), config.clone(), mint_authority.clone()],
        mint_authority_seeds,
    )?;

    Ok(())
}

// ── Twitter verification queue ────────────────────────────────────────────
// Layout: [8 disc][64 TwitterQueue struct][32*N Pubkeys]
//   TwitterQueue::HEADER_SIZE = 72  (disc + struct)
//   TwitterQueue::ENTRY_SIZE  = 32  (Pubkey)
//   capacity = (data_len - HEADER_SIZE) / ENTRY_SIZE
//   When len == capacity the account is extended by one slot before writing.

/// Append `entry` to the twitter verification queue PDA.
/// Creates the account (header-only, 0 capacity) on first call.
/// Skips silently if `entry` is already present.
/// Expands account data by one slot when at capacity.
pub fn queue_push<'a>(
    queue: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    program_id: &Pubkey,
    seeds: &[&[u8]],
    entry: &Pubkey,
) -> Result<()> {
    let header = TwitterQueue::HEADER_SIZE;
    let entry_sz = TwitterQueue::ENTRY_SIZE;

    if queue.lamports() == 0 {
        // First call: create header-only account (0 capacity, len=0).
        let (_, bump) = Pubkey::find_program_address(seeds, program_id);
        let bump_bytes = [bump];
        let mut full_seeds = seeds.to_vec();
        full_seeds.push(&bump_bytes);
        let signer_seeds: &[&[&[u8]]] = &[full_seeds.as_slice()];

        let lamports = Rent::get()?.minimum_balance(header);
        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                system_program.clone(),
                anchor_lang::system_program::CreateAccount {
                    from: payer.clone(),
                    to: queue.clone(),
                },
                signer_seeds,
            ),
            lamports,
            header as u64,
            program_id,
        )?;
        // Write zero_copy discriminator; len field stays 0 (zeroed by allocator).
        let mut data = queue.try_borrow_mut_data()?;
        data[0..8].copy_from_slice(&TwitterQueue::DISCRIMINATOR[..]);
    }

    // Read current len and capacity; deduplicate.
    let (len, capacity) = {
        let data = queue.try_borrow_data()?;
        if data.len() < header || data[0..8] != TwitterQueue::DISCRIMINATOR[..] {
            return Err(ProgramError::InvalidAccountData.into());
        }
        let len = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        let capacity = (data.len() - header) / entry_sz;
        for i in 0..len {
            let off = header + i * entry_sz;
            if data[off..off + entry_sz] == *entry.as_ref() {
                return Ok(());
            }
        }
        (len, capacity)
    };

    if len >= capacity {
        // At capacity — expand by one slot.
        let new_size = header + (len + 1) * entry_sz;
        queue.resize(new_size)?;
        let needed = Rent::get()?.minimum_balance(new_size);
        let current = queue.lamports();
        if needed > current {
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    system_program.clone(),
                    anchor_lang::system_program::Transfer {
                        from: payer.clone(),
                        to: queue.clone(),
                    },
                ),
                needed - current,
            )?;
        }
    }

    // Write entry at position `len` and update the len field.
    let mut data = queue.try_borrow_mut_data()?;
    let off = header + len * entry_sz;
    data[off..off + entry_sz].copy_from_slice(entry.as_ref());
    data[8..16].copy_from_slice(&(len as u64 + 1).to_le_bytes());

    Ok(())
}

/// Remove `entry` from the twitter verification queue PDA (swap-and-pop).
/// Shrinks the account by one slot and refunds excess rent to `recipient`.
/// Silently no-ops if the queue doesn't exist or entry isn't found.
pub fn queue_remove<'a>(
    queue: &AccountInfo<'a>,
    recipient: &AccountInfo<'a>,
    entry: &Pubkey,
) -> Result<()> {
    let header = TwitterQueue::HEADER_SIZE;
    let entry_sz = TwitterQueue::ENTRY_SIZE;

    if queue.lamports() == 0 {
        return Ok(());
    }

    let (len, idx) = {
        let data = queue.try_borrow_data()?;
        if data.len() < header || data[0..8] != TwitterQueue::DISCRIMINATOR[..] {
            return Ok(());
        }
        let len = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        let mut found = None;
        for i in 0..len {
            let off = header + i * entry_sz;
            if data[off..off + entry_sz] == *entry.as_ref() {
                found = Some(i);
                break;
            }
        }
        (len, found)
    };

    let idx = match idx {
        Some(i) => i,
        None => return Ok(()),
    };

    // Swap-and-pop then update len.
    {
        let mut data = queue.try_borrow_mut_data()?;
        if idx != len - 1 {
            let last_off = header + (len - 1) * entry_sz;
            let idx_off = header + idx * entry_sz;
            let last = data[last_off..last_off + entry_sz].to_vec();
            data[idx_off..idx_off + entry_sz].copy_from_slice(&last);
        }
        data[8..16].copy_from_slice(&(len as u64 - 1).to_le_bytes());
    }

    // Shrink account by one slot and refund rent.
    let new_size = header + (len - 1) * entry_sz;
    queue.resize(new_size)?;
    let new_min = Rent::get()?.minimum_balance(new_size);
    let current = queue.lamports();
    if current > new_min {
        **queue.try_borrow_mut_lamports()? -= current - new_min;
        **recipient.try_borrow_mut_lamports()? += current - new_min;
    }

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
