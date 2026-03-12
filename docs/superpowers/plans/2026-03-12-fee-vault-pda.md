# Fee Vault PDA Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace mutable `fee_recipient` with a program-controlled `fee_vault` PDA, delete `update_fee_recipient`, and add `withdraw_fees` instruction.

**Architecture:** Registration fees flow into a `fee_vault` PDA (seeds `[b"fee_vault"]`) instead of an external wallet. Admin withdraws fees via a new `withdraw_fees` instruction. The `fee_recipient` field in `ProgramConfig` is renamed to `fee_vault` (same memory position, layout-compatible).

**Tech Stack:** Rust/Anchor 0.32.1, Solana, TypeScript (tests)

**Spec:** `docs/superpowers/specs/2026-03-12-fee-vault-pda-design.md`

---

## Chunk 1: Core Changes (seeds, config, error, delete update_fee_recipient)

### Task 1: Add SEED_FEE_VAULT constant

**Files:**
- Modify: `programs/nara-agent-registry/src/seeds.rs`

- [ ] **Step 1: Add the constant**

In `programs/nara-agent-registry/src/seeds.rs`, add at the end:

```rust
pub const SEED_FEE_VAULT: &[u8] = b"fee_vault";
```

- [ ] **Step 2: Commit**

```bash
git add programs/nara-agent-registry/src/seeds.rs
git commit -m "feat: add SEED_FEE_VAULT constant"
```

### Task 2: Rename fee_recipient to fee_vault in ProgramConfig

**Files:**
- Modify: `programs/nara-agent-registry/src/state/program_config.rs`

- [ ] **Step 1: Rename the field**

In `programs/nara-agent-registry/src/state/program_config.rs`, change line 9:

```rust
// Before:
pub fee_recipient: Pubkey,
// After:
pub fee_vault: Pubkey,
```

- [ ] **Step 2: Commit**

```bash
git add programs/nara-agent-registry/src/state/program_config.rs
git commit -m "feat: rename fee_recipient to fee_vault in ProgramConfig"
```

### Task 3: Update error codes

**Files:**
- Modify: `programs/nara-agent-registry/src/error.rs`

- [ ] **Step 1: Replace InvalidFeeRecipient with InsufficientFeeVaultBalance**

In `programs/nara-agent-registry/src/error.rs`, replace:

```rust
// Remove:
#[msg("Fee recipient does not match config.fee_recipient")]
InvalidFeeRecipient,

// Add (in the same position to preserve error code ordering):
#[msg("Fee vault has insufficient balance for withdrawal")]
InsufficientFeeVaultBalance,
```

- [ ] **Step 2: Commit**

```bash
git add programs/nara-agent-registry/src/error.rs
git commit -m "feat: replace InvalidFeeRecipient with InsufficientFeeVaultBalance error"
```

### Task 4: Delete update_fee_recipient instruction

**Files:**
- Delete: `programs/nara-agent-registry/src/instructions/update_fee_recipient.rs`
- Modify: `programs/nara-agent-registry/src/instructions/mod.rs`
- Modify: `programs/nara-agent-registry/src/lib.rs`

- [ ] **Step 1: Delete the file**

```bash
rm programs/nara-agent-registry/src/instructions/update_fee_recipient.rs
```

- [ ] **Step 2: Remove from mod.rs**

In `programs/nara-agent-registry/src/instructions/mod.rs`, remove these two lines:

```rust
pub mod update_fee_recipient;
pub use update_fee_recipient::*;
```

- [ ] **Step 3: Remove from lib.rs**

In `programs/nara-agent-registry/src/lib.rs`, remove:

```rust
pub fn update_fee_recipient(ctx: Context<UpdateFeeRecipient>, new_recipient: Pubkey) -> Result<()> {
    instructions::update_fee_recipient::update_fee_recipient(ctx, new_recipient)
}
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: delete update_fee_recipient instruction"
```

### Task 5: Update init_config to compute and store fee_vault PDA

**Files:**
- Modify: `programs/nara-agent-registry/src/instructions/init_config.rs`

- [ ] **Step 1: Add fee_vault account to InitConfig struct**

Add after the `config` field (line 19) in the `InitConfig` struct:

```rust
/// CHECK: Fee vault PDA for holding registration fees; validated by seeds constraint.
#[account(mut, seeds = [SEED_FEE_VAULT], bump)]
pub fee_vault: UncheckedAccount<'info>,
```

- [ ] **Step 2: Update init_config handler to store fee_vault address**

In the `init_config` function, replace:

```rust
config.fee_recipient = ctx.accounts.admin.key();
```

with:

```rust
config.fee_vault = ctx.accounts.fee_vault.key();
```

- [ ] **Step 3: Commit**

```bash
git add programs/nara-agent-registry/src/instructions/init_config.rs
git commit -m "feat: init_config computes and stores fee_vault PDA"
```

## Chunk 2: Update registration instructions

### Task 6: Update register_agent to use fee_vault PDA

**Files:**
- Modify: `programs/nara-agent-registry/src/instructions/register_agent.rs`

- [ ] **Step 1: Update RegisterAgent struct**

Replace the `fee_recipient` account in `RegisterAgent` (lines 28-30):

```rust
// Before:
/// CHECK: must equal config.fee_recipient; validated in handler.
#[account(mut)]
pub fee_recipient: UncheckedAccount<'info>,

// After:
/// CHECK: Fee vault PDA; validated by seeds constraint.
#[account(mut, seeds = [SEED_FEE_VAULT], bump)]
pub fee_vault: UncheckedAccount<'info>,
```

- [ ] **Step 2: Update register_agent handler**

In the `register_agent` function, make these changes:

1. Remove the `require_keys_eq!` block (lines 38-42):
```rust
// Remove entirely:
require_keys_eq!(
    ctx.accounts.fee_recipient.key(),
    config.fee_recipient,
    AgentRegistryError::InvalidFeeRecipient
);
```

2. Simplify the transfer condition and update account reference (lines 47-57):
```rust
// Before:
if fee > 0 && ctx.accounts.fee_recipient.key() != ctx.accounts.authority.key() {
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.fee_recipient.to_account_info(),
            },
        ),
        fee,
    )?;
}

// After:
if fee > 0 {
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.fee_vault.to_account_info(),
            },
        ),
        fee,
    )?;
}
```

3. Remove the unused import `use crate::error::AgentRegistryError;` if no other usage remains in this file. Check first — `AgentRegistryError` is still used in `register_agent_with_referral` for `InvalidReferralAuthority`, so keep the import.

- [ ] **Step 3: Commit**

```bash
git add programs/nara-agent-registry/src/instructions/register_agent.rs
git commit -m "feat: register_agent sends fees to fee_vault PDA"
```

### Task 7: Update register_agent_with_referral to use fee_vault PDA

**Files:**
- Modify: `programs/nara-agent-registry/src/instructions/register_agent.rs`

- [ ] **Step 1: Update RegisterAgentWithReferral struct**

Replace the `fee_recipient` account in `RegisterAgentWithReferral` (lines 82-84):

```rust
// Before:
/// CHECK: must equal config.fee_recipient; validated in handler.
#[account(mut)]
pub fee_recipient: UncheckedAccount<'info>,

// After:
/// CHECK: Fee vault PDA; validated by seeds constraint.
#[account(mut, seeds = [SEED_FEE_VAULT], bump)]
pub fee_vault: UncheckedAccount<'info>,
```

- [ ] **Step 2: Update register_agent_with_referral handler**

1. Remove the `require_keys_eq!` block (lines 121-125):
```rust
// Remove entirely:
require_keys_eq!(
    ctx.accounts.fee_recipient.key(),
    config.fee_recipient,
    AgentRegistryError::InvalidFeeRecipient
);
```

2. Simplify system share transfer condition and update account reference (lines 145-157):
```rust
// Before:
// Transfer system's share to fee_recipient
if system_share > 0 && ctx.accounts.fee_recipient.key() != ctx.accounts.authority.key() {
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.fee_recipient.to_account_info(),
            },
        ),
        system_share,
    )?;
}

// After:
// Transfer system's share to fee_vault
if system_share > 0 {
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.fee_vault.to_account_info(),
            },
        ),
        system_share,
    )?;
}
```

- [ ] **Step 3: Commit**

```bash
git add programs/nara-agent-registry/src/instructions/register_agent.rs
git commit -m "feat: register_agent_with_referral sends system share to fee_vault PDA"
```

## Chunk 3: New withdraw_fees instruction

### Task 8: Create withdraw_fees instruction

**Files:**
- Create: `programs/nara-agent-registry/src/instructions/withdraw_fees.rs`
- Modify: `programs/nara-agent-registry/src/instructions/mod.rs`
- Modify: `programs/nara-agent-registry/src/lib.rs`

- [ ] **Step 1: Create the instruction file**

Create `programs/nara-agent-registry/src/instructions/withdraw_fees.rs`:

```rust
use anchor_lang::prelude::*;
use crate::state::ProgramConfig;
use crate::error::AgentRegistryError;
use crate::seeds::*;

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [SEED_CONFIG],
        bump,
        has_one = admin @ AgentRegistryError::Unauthorized,
    )]
    pub config: AccountLoader<'info, ProgramConfig>,
    /// CHECK: Fee vault PDA; validated by seeds constraint.
    #[account(mut, seeds = [SEED_FEE_VAULT], bump)]
    pub fee_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
    let vault_balance = ctx.accounts.fee_vault.lamports();
    let rent_exempt = Rent::get()?.minimum_balance(0);
    let available = vault_balance.saturating_sub(rent_exempt);

    require!(
        available >= amount,
        AgentRegistryError::InsufficientFeeVaultBalance
    );

    let fee_vault_bump = ctx.bumps.fee_vault;
    let signer_seeds: &[&[&[u8]]] = &[&[SEED_FEE_VAULT, &[fee_vault_bump]]];

    anchor_lang::system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.fee_vault.to_account_info(),
                to: ctx.accounts.admin.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    Ok(())
}
```

- [ ] **Step 2: Register in mod.rs**

In `programs/nara-agent-registry/src/instructions/mod.rs`, add (alphabetically):

```rust
pub mod withdraw_fees;
```

and:

```rust
pub use withdraw_fees::*;
```

- [ ] **Step 3: Register in lib.rs**

In `programs/nara-agent-registry/src/lib.rs`, add the instruction route (after `log_activity_with_referral`):

```rust
pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
    instructions::withdraw_fees::withdraw_fees(ctx, amount)
}
```

- [ ] **Step 4: Build to verify compilation**

Run: `anchor build`
Expected: successful build with no errors.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add withdraw_fees instruction for admin fee withdrawal"
```

## Chunk 4: Update tests

### Task 9: Update test helpers and PDA derivations

**Files:**
- Modify: `tests/nara-agent-registry.ts`

- [ ] **Step 1: Add feeVaultPDA helper**

In `tests/nara-agent-registry.ts`, after the existing PDA helpers (around line 73), add:

```typescript
const feeVaultPDA = (): PublicKey =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("fee_vault")],
    program.programId
  )[0];
```

- [ ] **Step 2: Update doRegisterAgent helper**

Replace the `doRegisterAgent` function (lines 149-157):

```typescript
async function doRegisterAgent(agentId: string) {
  await program.methods
    .registerAgent(agentId)
    .accounts({ feeVault: feeVaultPDA() })
    .rpc();
}
```

- [ ] **Step 3: Update doRegisterAgentWithReferral helper**

Replace the `doRegisterAgentWithReferral` function (lines 160-174):

```typescript
async function doRegisterAgentWithReferral(
  agentId: string,
  referralAgentKey: PublicKey,
  referralAuthorityKey: PublicKey,
) {
  await program.methods
    .registerAgentWithReferral(agentId)
    .accounts({
      feeVault: feeVaultPDA(),
      referralAgent: referralAgentKey,
      referralAuthority: referralAuthorityKey,
    })
    .rpc();
}
```

- [ ] **Step 4: Commit**

```bash
git add tests/nara-agent-registry.ts
git commit -m "test: update helpers to use feeVault PDA"
```

### Task 10: Update program_config tests

**Files:**
- Modify: `tests/nara-agent-registry.ts`

- [ ] **Step 1: Update init check**

In the `program_config` describe block, update the init test (line 187):

```typescript
// Before:
expect(cfg.feeRecipient.toBase58()).to.eq(authority.publicKey.toBase58());
// After:
expect(cfg.feeVault.toBase58()).to.eq(feeVaultPDA().toBase58());
```

- [ ] **Step 2: Replace update_fee_recipient test with withdraw_fees tests**

Remove the two `update_fee_recipient` tests (lines 215-258) and the "collects fee" test (lines 261-294). Replace with:

```typescript
it("collects fee to fee_vault PDA", async () => {
  const smallFee = new anchor.BN(10_000_000); // 0.01 SOL
  await program.methods
    .updateRegisterFee(smallFee)
    .accounts({})
    .rpc();

  try {
    const vaultKey = feeVaultPDA();
    const before = await provider.connection.getBalance(vaultKey);
    await doRegisterAgent("fee-vault-test-01");
    const after = await provider.connection.getBalance(vaultKey);
    expect(after - before).to.eq(10_000_000);
  } finally {
    await program.methods
      .updateRegisterFee(ONE_SOL)
      .accounts({})
      .rpc();
  }
});

it("withdraw_fees: admin can withdraw", async () => {
  const vaultKey = feeVaultPDA();
  const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(0);
  const vaultBalance = await provider.connection.getBalance(vaultKey);
  const available = vaultBalance - rentExempt;

  if (available > 0) {
    const adminBefore = await provider.connection.getBalance(authority.publicKey);
    await program.methods
      .withdrawFees(new anchor.BN(available))
      .accounts({})
      .rpc();
    const adminAfter = await provider.connection.getBalance(authority.publicKey);
    const vaultAfter = await provider.connection.getBalance(vaultKey);
    // Admin gained withdrawn amount (minus tx fee ~5000 lamports)
    expect(adminAfter).to.be.greaterThan(adminBefore + available - 100_000);
    // Vault is at rent-exempt minimum
    expect(vaultAfter).to.eq(rentExempt);
  }
});

it("withdraw_fees: rejects non-admin", async () => {
  const other = Keypair.generate();
  const sig = await provider.connection.requestAirdrop(
    other.publicKey,
    web3.LAMPORTS_PER_SOL
  );
  await provider.connection.confirmTransaction(sig);

  try {
    await program.methods
      .withdrawFees(new anchor.BN(1))
      .accounts({ admin: other.publicKey })
      .signers([other])
      .rpc();
    expect.fail("expected error");
  } catch (e: any) {
    expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
  }
});

it("withdraw_fees: rejects insufficient balance", async () => {
  const vaultKey = feeVaultPDA();
  const vaultBalance = await provider.connection.getBalance(vaultKey);
  const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(0);
  const tooMuch = vaultBalance - rentExempt + 1;

  try {
    await program.methods
      .withdrawFees(new anchor.BN(tooMuch))
      .accounts({})
      .rpc();
    expect.fail("expected error");
  } catch (e: any) {
    expect(e.error?.errorCode?.code ?? e.message).to.include(
      "InsufficientFeeVaultBalance"
    );
  }
});
```

- [ ] **Step 3: Update any remaining feeRecipient references in other tests**

Search the test file for any remaining `feeRecipient` references. The `doRegisterAgent` and `doRegisterAgentWithReferral` calls throughout the file that passed `feeRecipient` as a parameter should now work without it (since the helpers no longer take that param). Specifically check:

- Line 281: `await doRegisterAgent("fee-test-01", recipient.publicKey);` — this call site was in the old test we replaced, so it's already gone.
- Any other `doRegisterAgent` or `doRegisterAgentWithReferral` calls with extra args — remove the extra `feeRecipient` argument if present.

- [ ] **Step 4: Commit**

```bash
git add tests/nara-agent-registry.ts
git commit -m "test: update tests for fee_vault PDA and withdraw_fees"
```

### Task 11: Run tests and fix any issues

- [ ] **Step 1: Build the program**

Run: `anchor build`
Expected: successful build.

- [ ] **Step 2: Run the full test suite**

Run: `anchor test`
Expected: all tests pass.

- [ ] **Step 3: Fix any failures and commit**

If there are failures, fix them and commit with an appropriate message.

### Task 12: Update migrations/init.ts

**Files:**
- Modify: `migrations/init.ts`

- [ ] **Step 1: Update both feeRecipient references**

In `migrations/init.ts`, there are two `cfg.feeRecipient` references. Update both:

Line 66 (already-initialized branch):
```typescript
// Before:
console.log("  feeRecipient  :", cfg.feeRecipient.toBase58());
// After:
console.log("  feeVault      :", cfg.feeVault.toBase58());
```

Line 82 (just-initialized branch):
```typescript
// Before:
console.log("  feeRecipient  :", cfg.feeRecipient.toBase58());
// After:
console.log("  feeVault      :", cfg.feeVault.toBase58());
```

- [ ] **Step 2: Commit**

```bash
git add migrations/init.ts
git commit -m "chore: update migrations to use feeVault"
```
