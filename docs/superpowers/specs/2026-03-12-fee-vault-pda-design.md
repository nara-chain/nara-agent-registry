# Fee Vault PDA Design

## Summary

Replace the mutable `fee_recipient` external address in `ProgramConfig` with a program-controlled `fee_vault` PDA. Registration fees flow into this PDA instead of an external wallet. Admin can withdraw fees via a new `withdraw_fees` instruction. The `update_fee_recipient` instruction is deleted.

## Motivation

- Fee collection address should be deterministic and immutable (PDA), not a changeable external address
- Admin should be able to withdraw accumulated fees on demand
- Simplifies trust model: fees are held by the program, not sent to an arbitrary address

## Design

### 1. New PDA: fee_vault

- **Seeds**: `[b"fee_vault"]`
- **Type**: System account (holds SOL lamports)
- **Defined in**: `seeds.rs` as `pub const SEED_FEE_VAULT: &[u8] = b"fee_vault";`

### 2. ProgramConfig Change

Rename field `fee_recipient: Pubkey` to `fee_vault: Pubkey`. Memory layout unchanged (same position, same type).

```rust
#[account(zero_copy)]
#[repr(C)]
pub struct ProgramConfig {
    pub admin: Pubkey,
    pub fee_vault: Pubkey,  // was fee_recipient
    pub point_mint: Pubkey,
    pub referee_mint: Pubkey,
    pub referee_activity_mint: Pubkey,
    pub register_fee: u64,
    pub points_self: u64,
    pub points_referral: u64,
    pub referral_register_fee: u64,
    pub referral_fee_share: u64,
    pub referral_register_points: u64,
    pub activity_reward: u64,
    pub referral_activity_reward: u64,
    pub _reserved: [u8; 64],
}
```

### 3. init_config Changes

Compute fee_vault PDA and store in config:

```rust
config.fee_vault = Pubkey::find_program_address(&[SEED_FEE_VAULT], ctx.program_id).0;
```

### 4. Delete update_fee_recipient

Remove entirely:
- `instructions/update_fee_recipient.rs`
- Registration in `lib.rs`
- Re-exports in `instructions/mod.rs`

### 5. Modify register_agent

- Replace `fee_recipient` account with `fee_vault` account using seeds constraint:
  ```rust
  #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
  pub fee_vault: UncheckedAccount<'info>,
  ```
- Remove `require_keys_eq!` check against `config.fee_recipient`
- Transfer fee to `fee_vault` instead of `fee_recipient`

### 6. Modify register_agent_with_referral

Same changes as `register_agent`:
- `fee_recipient` account becomes `fee_vault` with seeds constraint
- System share of referral fee goes to `fee_vault`

### 7. New Instruction: withdraw_fees

```rust
pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()>
```

**Accounts**:
- `admin: Signer` - must be config.admin
- `config: AccountLoader<ProgramConfig>` - seeds = [SEED_CONFIG], has_one = admin
- `fee_vault: UncheckedAccount` - seeds = [SEED_FEE_VAULT], mut
- `system_program: Program<System>`

**Logic**:
1. Check fee_vault balance minus rent-exempt minimum >= amount
2. Transfer `amount` lamports from fee_vault to admin via PDA signer (CPI with seeds)
3. Error `InsufficientFeeVaultBalance` if insufficient funds

**Destination**: Always admin (the signer). No arbitrary recipient.

### 8. Error Code Changes

- Remove: `InvalidFeeRecipient`
- Add: `InsufficientFeeVaultBalance` - "Fee vault has insufficient balance for withdrawal"

### 9. Files Modified

| File | Action |
|------|--------|
| `seeds.rs` | Add `SEED_FEE_VAULT` |
| `state/program_config.rs` | Rename `fee_recipient` -> `fee_vault` |
| `instructions/init_config.rs` | Compute and store fee_vault PDA |
| `instructions/update_fee_recipient.rs` | **Delete** |
| `instructions/register_agent.rs` | fee_recipient -> fee_vault with seeds constraint |
| `instructions/mod.rs` | Remove update_fee_recipient, add withdraw_fees |
| `lib.rs` | Remove update_fee_recipient route, add withdraw_fees route |
| `error.rs` | Remove InvalidFeeRecipient, add InsufficientFeeVaultBalance |
| `instructions/withdraw_fees.rs` | **New file** |
| Tests | Update all fee-related tests |
| IDL | Regenerated after build |

### 10. Test Plan

- Registration fee lands in fee_vault PDA (not an external address)
- Referral registration: system share goes to fee_vault, referral share goes to referrer
- `withdraw_fees`: admin withdraws specified amount, balance decreases correctly
- `withdraw_fees`: non-admin caller is rejected with Unauthorized
- `withdraw_fees`: amount exceeding available balance (minus rent-exempt) is rejected
- `update_fee_recipient` no longer exists in the program
