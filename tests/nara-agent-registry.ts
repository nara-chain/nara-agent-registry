import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { NaraAgentRegistry } from "../target/types/nara_agent_registry";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { expect } from "chai";

// ── Constants matching Rust (derived from struct field sizes) ─────────────────
const DISC = 8;
const PUBKEY_SIZE = 32;
const RESERVED = 64;
const MEMORY_BUFFER_HEADER = DISC + PUBKEY_SIZE + PUBKEY_SIZE + 4 + 4 + RESERVED;
const AGENT_MEMORY_HEADER = DISC + PUBKEY_SIZE + RESERVED;
const BIO_META_HEADER = DISC + RESERVED;
const ONE_SOL = new anchor.BN(1_000_000_000);

describe("nara-agent-registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.NaraAgentRegistry as Program<NaraAgentRegistry>;
  const authority = provider.wallet as anchor.Wallet;

  // ── PDA helpers ─────────────────────────────────────────────────────────
  const agentPDA = (agentId: string): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("agent"), Buffer.from(agentId)],
      program.programId
    )[0];

  const bioPDA = (agentKey: PublicKey): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("bio"), agentKey.toBuffer()],
      program.programId
    )[0];

  const configPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      program.programId
    )[0];

  const metaPDA = (agentKey: PublicKey): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("meta"), agentKey.toBuffer()],
      program.programId
    )[0];

  // ── Utility: parse agent_id from zero-copy [u8;32] + u8 len ────────────
  function parseAgentId(agent: any): string {
    return Buffer.from(agent.agentId.slice(0, agent.agentIdLen)).toString("utf8");
  }

  // ── Utility: manually deserialize AgentBio / AgentMetadata ──────────────
  // Layout: [8 disc][64 reserved][4 len][bytes...]
  async function fetchBio(pda: PublicKey): Promise<string> {
    const info = await provider.connection.getAccountInfo(pda);
    if (!info) throw new Error("Bio account not found");
    const len = info.data.readUInt32LE(BIO_META_HEADER);
    const dataStart = BIO_META_HEADER + 4;
    return info.data.subarray(dataStart, dataStart + len).toString("utf8");
  }

  async function fetchMetadata(pda: PublicKey): Promise<string> {
    const info = await provider.connection.getAccountInfo(pda);
    if (!info) throw new Error("Metadata account not found");
    const len = info.data.readUInt32LE(BIO_META_HEADER);
    const dataStart = BIO_META_HEADER + 4;
    return info.data.subarray(dataStart, dataStart + len).toString("utf8");
  }

  // ── Utility: create a raw account owned by the program ──────────────────
  async function createProgramAccount(kp: Keypair, size: number) {
    const lamports =
      await provider.connection.getMinimumBalanceForRentExemption(size);
    const tx = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: authority.publicKey,
        newAccountPubkey: kp.publicKey,
        lamports,
        space: size,
        programId: program.programId,
      })
    );
    await provider.sendAndConfirm(tx, [kp]);
  }

  // ── Helper: register an agent with config + feeRecipient ─────────────────
  async function doRegisterAgent(
    agentId: string,
    feeRecipient: PublicKey = authority.publicKey,
  ) {
    await program.methods
      .registerAgent(agentId)
      .accountsStrict({
        authority: authority.publicKey,
        agent: agentPDA(agentId),
        config: configPDA(),
        feeRecipient,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  // ── One-time program init ────────────────────────────────────────────────
  before(async () => {
    await program.methods
      .initConfig()
      .accountsStrict({
        admin: authority.publicKey,
        config: configPDA(),
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  // ── program_config ────────────────────────────────────────────────────────
  describe("program_config", () => {
    it("initializes with admin and 1 SOL default fee", async () => {
      const cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.admin.toBase58()).to.eq(authority.publicKey.toBase58());
      expect(cfg.registerFee.eq(ONE_SOL)).to.be.true;
      expect(cfg.feeRecipient.toBase58()).to.eq(authority.publicKey.toBase58());
    });

    it("update_register_fee: admin can update", async () => {
      await program.methods
        .updateRegisterFee(new anchor.BN(0))
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();
      let cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.registerFee.toNumber()).to.eq(0);

      // Restore to 1 SOL
      await program.methods
        .updateRegisterFee(ONE_SOL)
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();
      cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.registerFee.eq(ONE_SOL)).to.be.true;
    });

    it("update_fee_recipient: admin can change and reset", async () => {
      const newRecipient = Keypair.generate();
      await program.methods
        .updateFeeRecipient(newRecipient.publicKey)
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();
      let cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.feeRecipient.toBase58()).to.eq(
        newRecipient.publicKey.toBase58()
      );

      // Reset to authority
      await program.methods
        .updateFeeRecipient(authority.publicKey)
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();
    });

    it("rejects non-admin on update_register_fee", async () => {
      const other = Keypair.generate();
      try {
        await program.methods
          .updateRegisterFee(new anchor.BN(0))
          .accountsStrict({ admin: other.publicKey, config: configPDA() })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("rejects non-admin on update_fee_recipient", async () => {
      const other = Keypair.generate();
      try {
        await program.methods
          .updateFeeRecipient(other.publicKey)
          .accountsStrict({ admin: other.publicKey, config: configPDA() })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("collects fee when fee_recipient differs from authority", async () => {
      const recipient = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(
        recipient.publicKey,
        web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);

      const smallFee = new anchor.BN(10_000_000); // 0.01 SOL
      await program.methods
        .updateRegisterFee(smallFee)
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();
      await program.methods
        .updateFeeRecipient(recipient.publicKey)
        .accountsStrict({ admin: authority.publicKey, config: configPDA() })
        .rpc();

      try {
        const before = await provider.connection.getBalance(recipient.publicKey);
        await doRegisterAgent("fee-test-01", recipient.publicKey);
        const after = await provider.connection.getBalance(recipient.publicKey);
        expect(after - before).to.eq(10_000_000);
      } finally {
        await program.methods
          .updateRegisterFee(ONE_SOL)
          .accountsStrict({ admin: authority.publicKey, config: configPDA() })
          .rpc();
        await program.methods
          .updateFeeRecipient(authority.publicKey)
          .accountsStrict({ admin: authority.publicKey, config: configPDA() })
          .rpc();
      }
    });
  });

  // ── register_agent ────────────────────────────────────────────────────────
  describe("register_agent", () => {
    const AGENT_ID = "test-agent-01";

    it("creates a new AgentRecord PDA", async () => {
      await doRegisterAgent(AGENT_ID);

      const agent = await program.account.agentRecord.fetch(agentPDA(AGENT_ID));
      expect(agent.authority.toBase58()).to.eq(authority.publicKey.toBase58());
      expect(parseAgentId(agent)).to.eq(AGENT_ID);
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
      expect(agent.memory.equals(PublicKey.default)).to.be.true;
      expect(agent.version).to.eq(0);
      expect(agent.createdAt.toNumber()).to.be.greaterThan(0);
      expect(agent.updatedAt.toNumber()).to.eq(0);
    });

    it("rejects duplicate agent IDs", async () => {
      try {
        await doRegisterAgent(AGENT_ID);
        expect.fail("expected error");
      } catch (_) {
        // Expected: account already in use
      }
    });

    it("rejects agent IDs shorter than 5 bytes (AgentIdTooShort)", async () => {
      try {
        await doRegisterAgent("abcd"); // 4 chars < 5 minimum
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("AgentIdTooShort");
      }
    });
  });

  // ── set_bio ─────────────────────────────────────────────────────────────
  describe("set_bio", () => {
    const AGENT_ID = "bio-agent-01";

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("creates the bio PDA on first call", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const bio = "I am an AI agent that writes beautiful haiku poems on demand.";
      await program.methods
        .setBio(AGENT_ID, bio)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          bioAccount: bioPDA(agentKey),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const b = await fetchBio(bioPDA(agentKey));
      expect(b).to.eq(bio);
    });

    it("updates the bio on subsequent calls (realloc)", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const newBio = "Short haiku generator.";
      await program.methods
        .setBio(AGENT_ID, newBio)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          bioAccount: bioPDA(agentKey),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const b = await fetchBio(bioPDA(agentKey));
      expect(b).to.eq(newBio);
    });

    it("rejects non-authority signer", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const other = Keypair.generate();
      try {
        await program.methods
          .setBio(AGENT_ID, "evil bio")
          .accountsStrict({
            authority: other.publicKey,
            agent: agentKey,
            bioAccount: bioPDA(agentKey),
            systemProgram: SystemProgram.programId,
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });
  });

  // ── set_metadata ────────────────────────────────────────────────────────
  describe("set_metadata", () => {
    const AGENT_ID = "meta-agent-01";

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("creates metadata PDA on first call and stores JSON", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const json = JSON.stringify({ tags: ["ai", "poetry"], lang: "en" });
      await program.methods
        .setMetadata(AGENT_ID, json)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          metadata: metaPDA(agentKey),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const meta = await fetchMetadata(metaPDA(agentKey));
      expect(meta).to.eq(json);
    });

    it("overwrites metadata on subsequent calls (realloc)", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const updated = JSON.stringify({ tags: ["ai"], lang: "zh", version: 2 });
      await program.methods
        .setMetadata(AGENT_ID, updated)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          metadata: metaPDA(agentKey),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const meta = await fetchMetadata(metaPDA(agentKey));
      expect(meta).to.eq(updated);
    });

    it("rejects non-authority signer", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const other = Keypair.generate();
      try {
        await program.methods
          .setMetadata(AGENT_ID, "{}")
          .accountsStrict({
            authority: other.publicKey,
            agent: agentKey,
            metadata: metaPDA(agentKey),
            systemProgram: SystemProgram.programId,
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });
  });

  // ── transfer_authority ────────────────────────────────────────────────────
  describe("transfer_authority", () => {
    const AGENT_ID = "transfer-agent-01";
    const newOwner = Keypair.generate();

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("transfers authority to a new pubkey", async () => {
      const agentKey = agentPDA(AGENT_ID);
      await program.methods
        .transferAuthority(AGENT_ID, newOwner.publicKey)
        .accountsStrict({ authority: authority.publicKey, agent: agentKey })
        .rpc();

      const agent = await program.account.agentRecord.fetch(agentKey);
      expect(agent.authority.toBase58()).to.eq(newOwner.publicKey.toBase58());
    });

    it("old authority can no longer modify", async () => {
      const agentKey = agentPDA(AGENT_ID);
      try {
        await program.methods
          .transferAuthority(AGENT_ID, authority.publicKey)
          .accountsStrict({ authority: authority.publicKey, agent: agentKey })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("rejects transfer while a pending buffer exists (HasPendingBuffer)", async () => {
      const agentId = "transfer-buf-01";
      const bufKp = Keypair.generate();
      const agentKey = agentPDA(agentId);

      await doRegisterAgent(agentId);
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + 10);
      await program.methods
        .initBuffer(agentId, 10)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufKp.publicKey,
        })
        .rpc();

      try {
        await program.methods
          .transferAuthority(agentId, Keypair.generate().publicKey)
          .accountsStrict({ authority: authority.publicKey, agent: agentKey })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "HasPendingBuffer"
        );
      }

      // Cleanup
      await program.methods
        .closeBuffer(agentId)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── buffer upload: new agent memory ─────────────────────────────────────
  describe("buffer upload (new memory)", () => {
    const AGENT_ID = "buffer-agent-01";
    const CONTENT = Buffer.from(
      "You are a professional poet specialising in haiku. " +
        "Write expressive, evocative poems that capture emotion and imagery " +
        "in exactly 17 syllables (5-7-5). " +
        "Focus on nature, impermanence, and sudden illumination. " +
        "Always respond with just the poem — no title, no explanation. " +
        "Example: 'An old silent pond / A frog jumps into the pond / Splash! Silence again.'"
    );

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("init_buffer → write ×2 → finalize_memory_new stores correct bytes", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const bufferKp = Keypair.generate();
      const memoryKp = Keypair.generate();
      const totalLen = CONTENT.length;

      await createProgramAccount(bufferKp, MEMORY_BUFFER_HEADER + totalLen);

      await program.methods
        .initBuffer(AGENT_ID, totalLen)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufferKp.publicKey,
        })
        .rpc();

      let agent = await program.account.agentRecord.fetch(agentKey);
      expect(agent.pendingBuffer.toBase58()).to.eq(
        bufferKp.publicKey.toBase58()
      );

      // Write in two chunks.
      const mid = Math.floor(totalLen / 2);
      await program.methods
        .writeToBuffer(AGENT_ID, 0, CONTENT.slice(0, mid))
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufferKp.publicKey,
        })
        .rpc();

      await program.methods
        .writeToBuffer(AGENT_ID, mid, CONTENT.slice(mid))
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufferKp.publicKey,
        })
        .rpc();

      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + totalLen);

      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufferKp.publicKey,
          newMemory: memoryKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      agent = await program.account.agentRecord.fetch(agentKey);
      expect(agent.memory.toBase58()).to.eq(memoryKp.publicKey.toBase58());
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
      expect(agent.version).to.eq(1);

      // Memory bytes match.
      const info = await provider.connection.getAccountInfo(memoryKp.publicKey);
      const stored = Buffer.from(info!.data.slice(AGENT_MEMORY_HEADER));
      expect(stored.toString()).to.eq(CONTENT.toString());

      // Buffer account closed.
      const bufInfo = await provider.connection.getAccountInfo(bufferKp.publicKey);
      expect(bufInfo).to.be.null;
    });
  });

  // ── write_to_buffer offset enforcement ───────────────────────────────────
  describe("write_to_buffer offset enforcement", () => {
    const AGENT_ID = "offset-err-01";
    let bufferKp: Keypair;

    before(async () => {
      bufferKp = Keypair.generate();
      await doRegisterAgent(AGENT_ID);
      await createProgramAccount(bufferKp, MEMORY_BUFFER_HEADER + 100);
      await program.methods
        .initBuffer(AGENT_ID, 100)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufferKp.publicKey,
        })
        .rpc();
    });

    it("rejects non-zero offset when cursor is 0 (OffsetMismatch)", async () => {
      try {
        await program.methods
          .writeToBuffer(AGENT_ID, 10, Buffer.alloc(10))
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: bufferKp.publicKey,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("OffsetMismatch");
      }
    });

    it("write at offset 0 succeeds and advances cursor to 10", async () => {
      await program.methods
        .writeToBuffer(AGENT_ID, 0, Buffer.alloc(10))
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufferKp.publicKey,
        })
        .rpc();

      const buf = await program.account.memoryBuffer.fetch(bufferKp.publicKey);
      expect(buf.writeOffset).to.eq(10);
    });

    it("retry at offset 0 is rejected (cursor already at 10)", async () => {
      try {
        await program.methods
          .writeToBuffer(AGENT_ID, 0, Buffer.alloc(10))
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: bufferKp.publicKey,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("OffsetMismatch");
      }
    });

    it("rejects write that would exceed total_len (WriteOutOfBounds)", async () => {
      try {
        await program.methods
          .writeToBuffer(AGENT_ID, 10, Buffer.alloc(95))
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: bufferKp.publicKey,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "WriteOutOfBounds"
        );
      }
    });
  });

  // ── init_buffer: rejects second buffer while one is pending ──────────────
  describe("init_buffer duplicate guard", () => {
    const AGENT_ID = "dup-buf-01";

    before(async () => {
      const bufKp = Keypair.generate();
      await doRegisterAgent(AGENT_ID);
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + 50);
      await program.methods
        .initBuffer(AGENT_ID, 50)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
    });

    it("rejects second init_buffer (PendingBufferExists)", async () => {
      const buf2 = Keypair.generate();
      await createProgramAccount(buf2, MEMORY_BUFFER_HEADER + 50);
      try {
        await program.methods
          .initBuffer(AGENT_ID, 50)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: buf2.publicKey,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "PendingBufferExists"
        );
      }
    });
  });

  // ── close_buffer ──────────────────────────────────────────────────────────
  describe("close_buffer", () => {
    const AGENT_ID = "close-buf-01";
    let buf1: Keypair;

    before(async () => {
      buf1 = Keypair.generate();
      await doRegisterAgent(AGENT_ID);
      await createProgramAccount(buf1, MEMORY_BUFFER_HEADER + 64);
      await program.methods
        .initBuffer(AGENT_ID, 64)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: buf1.publicKey,
        })
        .rpc();
    });

    it("rejects close by non-authority (Unauthorized)", async () => {
      const other = Keypair.generate();
      try {
        await program.methods
          .closeBuffer(AGENT_ID)
          .accountsStrict({
            authority: other.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: buf1.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("closes buffer and clears pending_buffer", async () => {
      await program.methods
        .closeBuffer(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: buf1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const agent = await program.account.agentRecord.fetch(agentPDA(AGENT_ID));
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
    });

    it("allows a fresh upload after close_buffer", async () => {
      const buf2 = Keypair.generate();
      await createProgramAccount(buf2, MEMORY_BUFFER_HEADER + 32);
      await program.methods
        .initBuffer(AGENT_ID, 32)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: buf2.publicKey,
        })
        .rpc();

      const agent = await program.account.agentRecord.fetch(agentPDA(AGENT_ID));
      expect(agent.pendingBuffer.toBase58()).to.eq(buf2.publicKey.toBase58());

      // Cleanup
      await program.methods
        .closeBuffer(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: buf2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── finalize_memory_new: incomplete buffer ────────────────────────────────
  describe("finalize_memory_new: incomplete buffer", () => {
    const AGENT_ID = "incomplete-buf-01";
    const TOTAL_LEN = 20;
    let bufKp: Keypair;
    let memoryKp: Keypair;

    before(async () => {
      bufKp = Keypair.generate();
      memoryKp = Keypair.generate();
      await doRegisterAgent(AGENT_ID);
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + TOTAL_LEN);
      await program.methods
        .initBuffer(AGENT_ID, TOTAL_LEN)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      // Write only half — buffer remains incomplete
      await program.methods
        .writeToBuffer(AGENT_ID, 0, Buffer.alloc(10))
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
    });

    it("rejects finalize when buffer is not fully written (BufferIncomplete)", async () => {
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + TOTAL_LEN);
      try {
        await program.methods
          .finalizeMemoryNew(AGENT_ID)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: bufKp.publicKey,
            newMemory: memoryKp.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "BufferIncomplete"
        );
      }

      // Cleanup
      await program.methods
        .closeBuffer(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── finalize_memory_update ─────────────────────────────────────────────────
  describe("finalize_memory_update", () => {
    const AGENT_ID = "update-01";
    const V1 = Buffer.from("Agent memory version 1.");
    const V2 = Buffer.from(
      "Agent memory version 2 — significantly longer than v1 to exercise rent accounting."
    );
    let memoryV1Kp: Keypair;

    before(async () => {
      memoryV1Kp = Keypair.generate();
      const bufKp = Keypair.generate();

      await doRegisterAgent(AGENT_ID);

      // Upload v1
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + V1.length);
      await program.methods
        .initBuffer(AGENT_ID, V1.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, V1)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await createProgramAccount(memoryV1Kp, AGENT_MEMORY_HEADER + V1.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
          newMemory: memoryV1Kp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });

    it("replaces memory and closes old memory account", async () => {
      const bufV2Kp = Keypair.generate();
      const memoryV2Kp = Keypair.generate();

      await createProgramAccount(bufV2Kp, MEMORY_BUFFER_HEADER + V2.length);
      await program.methods
        .initBuffer(AGENT_ID, V2.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufV2Kp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, V2)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufV2Kp.publicKey,
        })
        .rpc();
      await createProgramAccount(memoryV2Kp, AGENT_MEMORY_HEADER + V2.length);

      await program.methods
        .finalizeMemoryUpdate(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufV2Kp.publicKey,
          newMemory: memoryV2Kp.publicKey,
          oldMemory: memoryV1Kp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const agent = await program.account.agentRecord.fetch(agentPDA(AGENT_ID));
      expect(agent.memory.toBase58()).to.eq(memoryV2Kp.publicKey.toBase58());
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
      expect(agent.version).to.eq(2);

      const info = await provider.connection.getAccountInfo(memoryV2Kp.publicKey);
      expect(Buffer.from(info!.data.slice(AGENT_MEMORY_HEADER)).toString()).to.eq(
        V2.toString()
      );

      // Old memory account closed.
      const old = await provider.connection.getAccountInfo(memoryV1Kp.publicKey);
      expect(old).to.be.null;
    });

    it("rejects finalize_memory_new when memory already exists", async () => {
      const bufKp = Keypair.generate();
      const memoryKp = Keypair.generate();
      const tiny = Buffer.alloc(5, 0x42);

      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + tiny.length);
      await program.methods
        .initBuffer(AGENT_ID, tiny.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, tiny)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + tiny.length);

      try {
        await program.methods
          .finalizeMemoryNew(AGENT_ID)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(AGENT_ID),
            buffer: bufKp.publicKey,
            newMemory: memoryKp.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "MemoryAlreadyExists"
        );
      }

      // Cleanup
      await program.methods
        .closeBuffer(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });

    it("rejects finalize_memory_update when agent has no memory (MemoryNotFound)", async () => {
      const emptyId = "no-memory-01";
      const bufKp2 = Keypair.generate();
      const memoryKp2 = Keypair.generate();
      const dummyOldMemory = Keypair.generate();
      const data = Buffer.from("hello");

      await doRegisterAgent(emptyId);
      await createProgramAccount(bufKp2, MEMORY_BUFFER_HEADER + data.length);
      await program.methods
        .initBuffer(emptyId, data.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp2.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(emptyId, 0, data)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp2.publicKey,
        })
        .rpc();
      await createProgramAccount(memoryKp2, AGENT_MEMORY_HEADER + data.length);
      await createProgramAccount(dummyOldMemory, AGENT_MEMORY_HEADER + data.length);

      try {
        await program.methods
          .finalizeMemoryUpdate(emptyId)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(emptyId),
            buffer: bufKp2.publicKey,
            newMemory: memoryKp2.publicKey,
            oldMemory: dummyOldMemory.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "MemoryNotFound"
        );
      }

      // Cleanup
      await program.methods
        .closeBuffer(emptyId)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp2.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── finalize_memory_append ─────────────────────────────────────────────────
  describe("finalize_memory_append", () => {
    const AGENT_ID = "append-01";
    const INITIAL = Buffer.from("Initial memory content. ");
    const APPEND = Buffer.from("Appended memory content.");
    let memoryKp: Keypair;

    before(async () => {
      memoryKp = Keypair.generate();
      const bufKp = Keypair.generate();

      await doRegisterAgent(AGENT_ID);

      // Upload initial memory
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + INITIAL.length);
      await program.methods
        .initBuffer(AGENT_ID, INITIAL.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, INITIAL)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + INITIAL.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
          newMemory: memoryKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });

    it("appends buffer data to existing memory without allocating new account", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const appendBufKp = Keypair.generate();

      await createProgramAccount(appendBufKp, MEMORY_BUFFER_HEADER + APPEND.length);
      await program.methods
        .initBuffer(AGENT_ID, APPEND.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: appendBufKp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, APPEND)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: appendBufKp.publicKey,
        })
        .rpc();

      await program.methods
        .finalizeMemoryAppend(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: appendBufKp.publicKey,
          memory: memoryKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // AgentRecord updated.
      const agent = await program.account.agentRecord.fetch(agentKey);
      expect(agent.memory.toBase58()).to.eq(memoryKp.publicKey.toBase58()); // Same account!
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
      expect(agent.version).to.eq(2);

      // Memory contains initial + appended bytes.
      const info = await provider.connection.getAccountInfo(memoryKp.publicKey);
      const stored = Buffer.from(info!.data.slice(AGENT_MEMORY_HEADER));
      const expected = Buffer.concat([INITIAL, APPEND]);
      expect(stored.toString()).to.eq(expected.toString());

      // Buffer account closed.
      const bufInfo = await provider.connection.getAccountInfo(appendBufKp.publicKey);
      expect(bufInfo).to.be.null;
    });

    it("rejects append when agent has no memory (MemoryNotFound)", async () => {
      const emptyId = "no-mem-append";
      const bufKp = Keypair.generate();
      const data = Buffer.from("test");

      await doRegisterAgent(emptyId);
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + data.length);
      await program.methods
        .initBuffer(emptyId, data.length)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp.publicKey,
        })
        .rpc();
      await program.methods
        .writeToBuffer(emptyId, 0, data)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp.publicKey,
        })
        .rpc();

      try {
        await program.methods
          .finalizeMemoryAppend(emptyId)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentPDA(emptyId),
            buffer: bufKp.publicKey,
            memory: authority.publicKey, // dummy — agent has no memory
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include(
          "MemoryNotFound"
        );
      }

      // Cleanup
      await program.methods
        .closeBuffer(emptyId)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(emptyId),
          buffer: bufKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── delete_agent ─────────────────────────────────────────────────────────
  describe("delete_agent", () => {
    const AGENT_ID = "delete-agent-01";
    let memoryKp: Keypair;

    before(async () => {
      memoryKp = Keypair.generate();
      const bufKp = Keypair.generate();
      const CONTENT = Buffer.from("agent memory to be deleted");

      await doRegisterAgent(AGENT_ID);

      // Set bio and metadata.
      await program.methods
        .setBio(AGENT_ID, "An agent that will be deleted.")
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          bioAccount: bioPDA(agentPDA(AGENT_ID)),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods
        .setMetadata(AGENT_ID, JSON.stringify({ tag: "temp" }))
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          metadata: metaPDA(agentPDA(AGENT_ID)),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Upload memory.
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + CONTENT.length);
      await program.methods
        .initBuffer(AGENT_ID, CONTENT.length)
        .accountsStrict({ authority: authority.publicKey, agent: agentPDA(AGENT_ID), buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, CONTENT)
        .accountsStrict({ authority: authority.publicKey, agent: agentPDA(AGENT_ID), buffer: bufKp.publicKey })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + CONTENT.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          buffer: bufKp.publicKey,
          newMemory: memoryKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });

    it("closes agent record, bio, metadata, and memory; returns rent", async () => {
      const agentKey = agentPDA(AGENT_ID);

      await program.methods
        .deleteAgent(AGENT_ID)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          bio: bioPDA(agentKey),
          metadata: metaPDA(agentKey),
          memoryAccount: memoryKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      expect(await provider.connection.getAccountInfo(agentKey)).to.be.null;
      expect(await provider.connection.getAccountInfo(bioPDA(agentKey))).to.be.null;
      expect(await provider.connection.getAccountInfo(metaPDA(agentKey))).to.be.null;
      expect(await provider.connection.getAccountInfo(memoryKp.publicKey)).to.be.null;
    });

    it("allows re-registration with the same agent_id after deletion", async () => {
      const agentKey = agentPDA(AGENT_ID);
      await doRegisterAgent(AGENT_ID);

      const agent = await program.account.agentRecord.fetch(agentKey);
      expect(parseAgentId(agent)).to.eq(AGENT_ID);
      expect(agent.version).to.eq(0);
    });

    it("rejects non-authority (Unauthorized)", async () => {
      const other = Keypair.generate();
      const agentKey = agentPDA(AGENT_ID);
      try {
        await program.methods
          .deleteAgent(AGENT_ID)
          .accountsStrict({
            authority: other.publicKey,
            agent: agentKey,
            bio: bioPDA(agentKey),
            metadata: metaPDA(agentKey),
            memoryAccount: authority.publicKey, // no memory, dummy
            systemProgram: SystemProgram.programId,
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("rejects deletion while a pending buffer exists (HasPendingBuffer)", async () => {
      const agentId3 = "del-buf-guard";
      const bufKp = Keypair.generate();
      const agentKey = agentPDA(agentId3);

      await doRegisterAgent(agentId3);
      await createProgramAccount(bufKp, MEMORY_BUFFER_HEADER + 10);
      await program.methods
        .initBuffer(agentId3, 10)
        .accountsStrict({ authority: authority.publicKey, agent: agentKey, buffer: bufKp.publicKey })
        .rpc();

      try {
        await program.methods
          .deleteAgent(agentId3)
          .accountsStrict({
            authority: authority.publicKey,
            agent: agentKey,
            bio: bioPDA(agentKey),
            metadata: metaPDA(agentKey),
            memoryAccount: authority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("HasPendingBuffer");
      }

      // Cleanup
      await program.methods
        .closeBuffer(agentId3)
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
          buffer: bufKp.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    });
  });

  // ── log_activity ──────────────────────────────────────────────────────────
  describe("log_activity", () => {
    const AGENT_ID = "log-agent-01";

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("emits ActivityLogged event", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const listener = program.addEventListener("activityLogged", (event) => {
        expect(event.agentId).to.eq(AGENT_ID);
        expect(event.model).to.eq("gpt-4");
        expect(event.activity).to.eq("chat");
        expect(event.log).to.eq("handled user query about weather");
        expect(event.authority.toBase58()).to.eq(authority.publicKey.toBase58());
        expect(event.timestamp.toNumber()).to.be.greaterThan(0);
      });

      await program.methods
        .logActivity(
          AGENT_ID,
          "gpt-4",
          "chat",
          "handled user query about weather",
        )
        .accountsStrict({
          authority: authority.publicKey,
          agent: agentKey,
        })
        .rpc();

      // Give the event listener time to fire
      await new Promise((resolve) => setTimeout(resolve, 2000));
      program.removeEventListener(listener);
    });

    it("rejects non-authority signer", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const other = Keypair.generate();
      try {
        await program.methods
          .logActivity(AGENT_ID, "gpt-4", "chat", "evil log")
          .accountsStrict({
            authority: other.publicKey,
            agent: agentKey,
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });
  });
});
