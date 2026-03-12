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

**Distinction from treasury PDA**: The program has two SOL-holding PDAs with different roles:
- `treasury` (`[b"treasury"]`): externally funded, disburses activity rewards to users
- `fee_vault` (`[b"fee_vault"]`): accumulates registration fees from registrants, withdrawn by admin

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

Add `fee_vault` as an account in `InitConfig` so it gets created on-chain during initialization:

```rust
/// CHECK: Fee vault PDA for holding registration fees; validated by seeds constraint.
#[account(mut, seeds = [SEED_FEE_VAULT], bump)]
pub fee_vault: UncheckedAccount<'info>,
```

Compute fee_vault PDA address and store in config:

```rust
config.fee_vault = ctx.accounts.fee_vault.key();
```

The fee_vault account does not need explicit `create_account` -- it receives lamports via `system_program::transfer` from registrants. The rent-exempt minimum for a 0-byte account (~890,880 lamports) is negligible and will be covered by the first registration fee.

### 4. Delete update_fee_recipient

Remove entirely:
- `instructions/update_fee_recipient.rs`
- Registration in `lib.rs`
- Re-exports in `instructions/mod.rs`

### 5. Modify register_agent

- Replace `fee_recipient` account with `fee_vault` account using seeds constraint:
  ```rust
  /// CHECK: Fee vault PDA; validated by seeds constraint.
  #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
  pub fee_vault: UncheckedAccount<'info>,
  ```
- Remove `require_keys_eq!` check against `config.fee_recipient` (no longer needed -- seeds constraint guarantees the correct PDA)
- Simplify transfer condition from `if fee > 0 && fee_recipient != authority` to `if fee > 0` (the PDA can never equal the authority, so the self-payment guard is unnecessary)
- Transfer fee to `fee_vault` instead of `fee_recipient`

### 6. Modify register_agent_with_referral

Same changes as `register_agent`:
- `fee_recipient` account becomes `fee_vault` with seeds constraint and `/// CHECK:` comment
- Simplify transfer condition (remove self-payment guard)
- System share of referral fee goes to `fee_vault`

### 7. New Instruction: withdraw_fees

```rust
pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()>
```

**Accounts**:
- `admin: Signer` - must be config.admin
- `config: AccountLoader<ProgramConfig>` - seeds = [SEED_CONFIG], has_one = admin
- `fee_vault: UncheckedAccount` - seeds = [SEED_FEE_VAULT], mut, with `/// CHECK:` safety comment
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
- `withdraw_fees`: withdraw when vault has exactly rent-exempt minimum (available = 0) is rejected
- `withdraw_fees`: withdraw exact available balance succeeds (boundary test)
- `update_fee_recipient` no longer exists in the program
