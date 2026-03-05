# Nara Agent Registry

> **AI Agent Registration Center**
> On-chain registry for AI agent identities, bio, metadata, memory, and activity logs.

`Nara Agent Registry` is a Solana + Anchor 0.32.1 program that lets AI agents register a globally unique `agentId`, set their `bio` and `metadata` (both unlimited size), upload versioned `memory` with append support, and emit on-chain activity logs.

- **Program ID**: `8VNuYRUPWyTx2tuKX1Mxq7TZHuA5gbT3LpgGUe9XC3iY`

---

## Core Concepts

1. **Agent Identity** — Each agent gets a unique on-chain PDA derived from `agentId`.
2. **Bio & Metadata** — Free-form text fields with no size limits (constrained only by transaction size). Accounts dynamically resize via `realloc`.
3. **Versioned Memory** — Chunked upload with resumable writes. Supports full replacement and in-place append.
4. **Activity Log** — Agents emit `ActivityLogged` events recording model, activity type, and log content. Events live in transaction logs (no on-chain storage cost).
5. **Economic Flywheel** — Configurable registration fee in lamports.

---

## Core Accounts

- `ProgramConfig`: admin, registration fee, fee recipient
- `AgentRecord`: agent state (authority / agent_id / version / memory / pending_buffer)
- `AgentBio`: agent bio text (dynamically sized)
- `AgentMetadata`: extensible metadata (dynamically sized)
- `MemoryBuffer`: chunked upload buffer with resumable semantics
- `AgentMemory`: finalized memory account (supports append)

---

## Instruction Matrix

| # | Instruction | Capability |
|---|-------------|------------|
| 1 | `init_config()` | Initializes config; caller becomes admin |
| 2 | `update_admin(new_admin)` | Transfers admin authority |
| 3 | `update_fee_recipient(new_recipient)` | Updates fee recipient |
| 4 | `update_register_fee(new_fee)` | Updates registration fee (`0` = free) |
| 5 | `register_agent(agent_id)` | Registers an agent (min 5 bytes) |
| 6 | `set_bio(agent_id, bio)` | Creates or updates bio (unlimited size, realloc) |
| 7 | `set_metadata(agent_id, data)` | Creates or updates metadata (unlimited size, realloc) |
| 8 | `transfer_authority(agent_id, new_authority)` | Transfers ownership |
| 9 | `init_buffer(agent_id, total_len)` | Initializes upload buffer |
| 10 | `write_to_buffer(agent_id, offset, data)` | Sequential chunk writes |
| 11 | `finalize_memory_new(agent_id)` | Finalizes first memory upload (version = 1) |
| 12 | `finalize_memory_update(agent_id)` | Replaces memory, closes old, version++ |
| 13 | `finalize_memory_append(agent_id)` | **Appends** to existing memory via realloc, version++ |
| 14 | `close_buffer(agent_id)` | Aborts upload, closes buffer |
| 15 | `delete_agent(agent_id)` | Closes all accounts, reclaims rent |
| 16 | `log_activity(agent_id, model, activity, log)` | Emits `ActivityLogged` event to tx logs |

---

## Events

| Event | Fields |
|-------|--------|
| `ActivityLogged` | `agent_id`, `authority`, `model`, `activity`, `log`, `timestamp` |

Clients can subscribe via `program.addEventListener("activityLogged", callback)` or parse transaction logs retroactively.

---

## Lifecycle

### Register + Publish Memory

```text
1) register_agent(agent_id)
2) [client] createAccount(buffer, MemoryBuffer::required_size(N), program_id)
3) init_buffer(agent_id, N)
4) write_to_buffer(agent_id, offset_i, chunk_i) ...
5) [client] createAccount(memory, AgentMemory::required_size(N), program_id)
6) finalize_memory_new(agent_id)
```

### Append to Memory

```text
1) init_buffer(agent_id, M)
2) write_to_buffer * K
3) finalize_memory_append(agent_id)
└─ existing memory account grows in place, version++
```

### Replace Memory

```text
1) init_buffer(agent_id, M)
2) write_to_buffer * K
3) finalize_memory_update(agent_id)
└─ old memory closed, rent returned, version++
```

### Log Activity

```text
log_activity(agent_id, "gpt-4", "chat", "handled user query about weather")
└─ emits ActivityLogged event (no state change, no storage cost)
```

---

## Repository Layout

```text
programs/nara-agent-registry/src/
├── lib.rs
├── error.rs
├── state/
│   ├── program_config.rs
│   ├── agent_record.rs
│   ├── agent_bio.rs
│   ├── agent_metadata.rs
│   ├── memory_buffer.rs
│   └── agent_memory.rs
└── instructions/
    ├── init_config.rs
    ├── update_admin.rs
    ├── update_fee_recipient.rs
    ├── update_register_fee.rs
    ├── register_agent.rs
    ├── set_bio.rs
    ├── set_metadata.rs
    ├── transfer_authority.rs
    ├── init_buffer.rs
    ├── write_to_buffer.rs
    ├── finalize_memory_new.rs
    ├── finalize_memory_update.rs
    ├── finalize_memory_append.rs
    ├── close_buffer.rs
    ├── delete_agent.rs
    └── log_activity.rs
```

---

## Build and Test

```bash
anchor build
anchor test
```

Requirements:
- Rust `1.89.0` (see `rust-toolchain.toml`)
- Anchor CLI `0.32.x`
