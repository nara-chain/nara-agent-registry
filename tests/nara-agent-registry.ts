import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { NaraAgentRegistry } from "../target/types/nara_agent_registry";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getAccount,
} from "@solana/spl-token";
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

  const pointMintPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("point_mint")],
      program.programId
    )[0];

  const refereeMintPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("referee_mint")],
      program.programId
    )[0];

  const refereeActivityMintPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("referee_activity_mint")],
      program.programId
    )[0];

  const feeVaultPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("fee_vault")],
      program.programId
    )[0];

  const twitterPDA = (agentKey: PublicKey): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("twitter"), agentKey.toBuffer()],
      program.programId
    )[0];

  const twitterHandlePDA = (username: string): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("twitter_handle"), Buffer.from(username)],
      program.programId
    )[0];

  const twitterVerifyVaultPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("twitter_verify_vault")],
      program.programId
    )[0];

  const twitterQueuePDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("twitter_queue")],
      program.programId
    )[0];

  const tweetVerifyPDA = (agentKey: PublicKey): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("tweet_verify"), agentKey.toBuffer()],
      program.programId
    )[0];

  const tweetVerifyQueuePDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("tweet_verify_queue")],
      program.programId
    )[0];

  /** Read the list of Pubkeys in the twitter verification queue PDA.
   *  Layout: [8 disc][64 TwitterQueue struct (len at offset 8)][32*N Pubkeys starting at 72]
   */
  const readTwitterQueue = async (): Promise<PublicKey[]> => {
    const QUEUE_HEADER = 16; // 8 disc + 8 struct (len: u64)
    const info = await provider.connection.getAccountInfo(twitterQueuePDA());
    if (!info || info.data.length < QUEUE_HEADER) return [];
    const len = Number(info.data.readBigUInt64LE(8));
    const entries: PublicKey[] = [];
    for (let i = 0; i < len; i++) {
      const off = QUEUE_HEADER + i * 32;
      entries.push(new PublicKey(info.data.slice(off, off + 32)));
    }
    return entries;
  };

  const readTweetVerifyQueue = async (): Promise<PublicKey[]> => {
    const QUEUE_HEADER = 16;
    const info = await provider.connection.getAccountInfo(tweetVerifyQueuePDA());
    if (!info || info.data.length < QUEUE_HEADER) return [];
    const len = Number(info.data.readBigUInt64LE(8));
    const entries: PublicKey[] = [];
    for (let i = 0; i < len; i++) {
      const off = QUEUE_HEADER + i * 32;
      entries.push(new PublicKey(info.data.slice(off, off + 32)));
    }
    return entries;
  };

  const treasuryPDA = (): PublicKey =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("treasury")],
      program.programId
    )[0];

  // ── Utility: parse agent_id from zero-copy [u8;32] + u32 len ────────────
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

  // ── Utility: get point token balance for a wallet ──────────────────────
  async function getPointBalance(wallet: PublicKey): Promise<bigint> {
    const mint = pointMintPDA();
    const ata = getAssociatedTokenAddressSync(mint, wallet, true, TOKEN_2022_PROGRAM_ID);
    try {
      const account = await getAccount(provider.connection, ata, undefined, TOKEN_2022_PROGRAM_ID);
      return account.amount;
    } catch {
      return BigInt(0);
    }
  }

  async function getRefereeBalance(wallet: PublicKey): Promise<bigint> {
    const mint = refereeMintPDA();
    const ata = getAssociatedTokenAddressSync(mint, wallet, true, TOKEN_2022_PROGRAM_ID);
    try {
      const account = await getAccount(provider.connection, ata, undefined, TOKEN_2022_PROGRAM_ID);
      return account.amount;
    } catch {
      return BigInt(0);
    }
  }

  async function getRefereeActivityBalance(wallet: PublicKey): Promise<bigint> {
    const mint = refereeActivityMintPDA();
    const ata = getAssociatedTokenAddressSync(mint, wallet, true, TOKEN_2022_PROGRAM_ID);
    try {
      const account = await getAccount(provider.connection, ata, undefined, TOKEN_2022_PROGRAM_ID);
      return account.amount;
    } catch {
      return BigInt(0);
    }
  }

  // ── Helper: register an agent (no referral) ─────────────────────────────
  async function doRegisterAgent(agentId: string) {
    await program.methods
      .registerAgent(agentId)
      .accounts({ feeVault: feeVaultPDA() })
      .rpc();
  }

  // ── Helper: register an agent with referral ────────────────────────────
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

  // ── One-time program init ────────────────────────────────────────────────
  before(async () => {
    await program.methods.initConfig().rpc();
  });

  // ── program_config ────────────────────────────────────────────────────────
  describe("program_config", () => {
    it("initializes with admin, 1 SOL default fee, and point mint", async () => {
      const cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.admin.toBase58()).to.eq(authority.publicKey.toBase58());
      expect(cfg.registerFee.eq(ONE_SOL)).to.be.true;
      expect(cfg.feeVault.toBase58()).to.eq(feeVaultPDA().toBase58());
      expect(cfg.pointMint.toBase58()).to.eq(pointMintPDA().toBase58());
    });

    it("point mint is a Token2022 account", async () => {
      const mint = pointMintPDA();
      const info = await provider.connection.getAccountInfo(mint);
      expect(info).to.not.be.null;
      expect(info!.owner.toBase58()).to.eq(TOKEN_2022_PROGRAM_ID.toBase58());
    });

    it("update_register_fee: admin can update", async () => {
      await program.methods
        .updateRegisterFee(new anchor.BN(0))
        .accounts({})
        .rpc();
      let cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.registerFee.toNumber()).to.eq(0);

      // Restore to 1 SOL
      await program.methods
        .updateRegisterFee(ONE_SOL)
        .accounts({})
        .rpc();
      cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.registerFee.eq(ONE_SOL)).to.be.true;
    });

    it("rejects non-admin on update_register_fee", async () => {
      const other = Keypair.generate();
      try {
        await program.methods
          .updateRegisterFee(new anchor.BN(0))
          .accounts({ admin: other.publicKey })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

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

    describe("expand_config", () => {
      // ProgramConfig struct = 544 bytes, + 8 discriminator = 552 bytes on-chain
      const INITIAL_CONFIG_SIZE = 552;
      const EXTEND_SIZE = 128;

      it("expands account data by extend_size bytes", async () => {
        const config = configPDA();
        const before = await provider.connection.getAccountInfo(config);
        expect(before!.data.length).to.eq(INITIAL_CONFIG_SIZE);

        await program.methods
          .expandConfig(new anchor.BN(EXTEND_SIZE))
          .accounts({})
          .rpc();

        const after = await provider.connection.getAccountInfo(config);
        expect(after!.data.length).to.eq(INITIAL_CONFIG_SIZE + EXTEND_SIZE);
      });

      it("on-chain instructions execute correctly after expansion: set_twitter + verify_twitter", async () => {
        const AGENT_ID = "expand-cfg-twitter-test";
        const USERNAME = "expand_test_user";
        const TWEET_URL = "https://x.com/expand_test_user/status/999";
        const FEE = new anchor.BN(5_000_000); // 0.005 SOL

        // Register a fresh agent (temporarily set fee=0)
        await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
        await doRegisterAgent(AGENT_ID);
        await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

        // Set up a local verifier
        const expandVerifier = Keypair.generate();
        const sig = await provider.connection.requestAirdrop(
          expandVerifier.publicKey,
          2 * web3.LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(sig);

        // Write twitter config fields on-chain (proves config writable after expansion)
        await program.methods
          .updateTwitterVerifier(expandVerifier.publicKey)
          .accounts({})
          .rpc();
        await program.methods
          .updateTwitterVerificationConfig(FEE, new anchor.BN(0), new anchor.BN(0))
          .accounts({})
          .rpc();

        // Verify config fields read back correctly
        const cfg = await program.account.programConfig.fetch(configPDA());
        expect(cfg.twitterVerifier.toBase58()).to.eq(expandVerifier.publicKey.toBase58());
        expect(cfg.twitterVerificationFee.toNumber()).to.eq(FEE.toNumber());

        // set_twitter: agent pays fee into vault
        const vaultBefore = await provider.connection.getBalance(twitterVerifyVaultPDA());
        await program.methods
          .setTwitter(AGENT_ID, USERNAME, TWEET_URL)
          .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
          .rpc();
        const vaultAfter = await provider.connection.getBalance(twitterVerifyVaultPDA());
        expect(vaultAfter - vaultBefore).to.eq(FEE.toNumber());

        const agentKey = agentPDA(AGENT_ID);
        const twitterKey = twitterPDA(agentKey);
        const pendingAcc = await program.account.agentTwitter.fetch(twitterKey);
        expect(pendingAcc.status.toNumber()).to.eq(1); // Pending

        // verify_twitter: verifier approves, fee refunded
        await program.methods
          .verifyTwitter(AGENT_ID, USERNAME)
          .accounts({
            verifier: expandVerifier.publicKey,
            authority: authority.publicKey,
            twitterVerifyVault: twitterVerifyVaultPDA(),
            treasury: treasuryPDA(),
          })
          .signers([expandVerifier])
          .rpc();

        const twitterAcc = await program.account.agentTwitter.fetch(twitterKey);
        expect(twitterAcc.status.toNumber()).to.eq(2); // Verified
        expect(twitterAcc.verifiedAt.toNumber()).to.be.greaterThan(0);

        // TwitterHandle PDA created, bound to correct agent
        const handleAcc = await program.account.twitterHandle.fetch(twitterHandlePDA(USERNAME));
        expect(handleAcc.agent.toBase58()).to.eq(agentPDA(AGENT_ID).toBase58());
      });

      it("rejects non-admin on expand_config", async () => {
        const other = Keypair.generate();
        const sig = await provider.connection.requestAirdrop(
          other.publicKey,
          web3.LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(sig);

        try {
          await program.methods
            .expandConfig(new anchor.BN(64))
            .accounts({ admin: other.publicKey })
            .signers([other])
            .rpc();
          expect.fail("expected error");
        } catch (e: any) {
          expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
        }
      });

      it("rejects extend_size = 0", async () => {
        try {
          await program.methods
            .expandConfig(new anchor.BN(0))
            .accounts({})
            .rpc();
          expect.fail("expected error");
        } catch (e: any) {
          expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
        }
      });
    });
  });

  // ── register_agent ────────────────────────────────────────────────────────
  describe("register_agent", () => {
    const AGENT_ID = "test-agent-01";

    it("creates a new AgentState PDA", async () => {
      await doRegisterAgent(AGENT_ID);

      const agent = await program.account.agentState.fetch(agentPDA(AGENT_ID));
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
        .accounts({})
        .rpc();

      const b = await fetchBio(bioPDA(agentKey));
      expect(b).to.eq(bio);
    });

    it("updates the bio on subsequent calls (realloc)", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const newBio = "Short haiku generator.";
      await program.methods
        .setBio(AGENT_ID, newBio)
        .accounts({})
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
          .accounts({ authority: other.publicKey })
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
        .accounts({})
        .rpc();

      const meta = await fetchMetadata(metaPDA(agentKey));
      expect(meta).to.eq(json);
    });

    it("overwrites metadata on subsequent calls (realloc)", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const updated = JSON.stringify({ tags: ["ai"], lang: "zh", version: 2 });
      await program.methods
        .setMetadata(AGENT_ID, updated)
        .accounts({})
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
          .accounts({ authority: other.publicKey })
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
        .accounts({})
        .rpc();

      const agent = await program.account.agentState.fetch(agentKey);
      expect(agent.authority.toBase58()).to.eq(newOwner.publicKey.toBase58());
    });

    it("old authority can no longer modify", async () => {
      const agentKey = agentPDA(AGENT_ID);
      try {
        await program.methods
          .transferAuthority(AGENT_ID, authority.publicKey)
          .accounts({})
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();

      try {
        await program.methods
          .transferAuthority(agentId, Keypair.generate().publicKey)
          .accounts({})
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
        .accounts({ buffer: bufKp.publicKey })
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
        .accounts({ buffer: bufferKp.publicKey })
        .rpc();

      let agent = await program.account.agentState.fetch(agentKey);
      expect(agent.pendingBuffer.toBase58()).to.eq(
        bufferKp.publicKey.toBase58()
      );

      // Write in two chunks.
      const mid = Math.floor(totalLen / 2);
      await program.methods
        .writeToBuffer(AGENT_ID, 0, CONTENT.slice(0, mid))
        .accounts({ buffer: bufferKp.publicKey })
        .rpc();

      await program.methods
        .writeToBuffer(AGENT_ID, mid, CONTENT.slice(mid))
        .accounts({ buffer: bufferKp.publicKey })
        .rpc();

      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + totalLen);

      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accounts({ buffer: bufferKp.publicKey, newMemory: memoryKp.publicKey })
        .rpc();

      agent = await program.account.agentState.fetch(agentKey);
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
        .accounts({ buffer: bufferKp.publicKey })
        .rpc();
    });

    it("rejects non-zero offset when cursor is 0 (OffsetMismatch)", async () => {
      try {
        await program.methods
          .writeToBuffer(AGENT_ID, 10, Buffer.alloc(10))
          .accounts({
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
        .accounts({ buffer: bufferKp.publicKey })
        .rpc();

      const buf = await program.account.memoryBuffer.fetch(bufferKp.publicKey);
      expect(buf.writeOffset).to.eq(10);
    });

    it("retry at offset 0 is rejected (cursor already at 10)", async () => {
      try {
        await program.methods
          .writeToBuffer(AGENT_ID, 0, Buffer.alloc(10))
          .accounts({
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
          .accounts({
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
    });

    it("rejects second init_buffer (PendingBufferExists)", async () => {
      const buf2 = Keypair.generate();
      await createProgramAccount(buf2, MEMORY_BUFFER_HEADER + 50);
      try {
        await program.methods
          .initBuffer(AGENT_ID, 50)
          .accounts({ buffer: buf2.publicKey })
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
        .accounts({ buffer: buf1.publicKey })
        .rpc();
    });

    it("rejects close by non-authority (Unauthorized)", async () => {
      const other = Keypair.generate();
      try {
        await program.methods
          .closeBuffer(AGENT_ID)
          .accounts({ authority: other.publicKey, buffer: buf1.publicKey } as any)
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
        .accounts({ buffer: buf1.publicKey })
        .rpc();

      const agent = await program.account.agentState.fetch(agentPDA(AGENT_ID));
      expect(agent.pendingBuffer.equals(PublicKey.default)).to.be.true;
    });

    it("allows a fresh upload after close_buffer", async () => {
      const buf2 = Keypair.generate();
      await createProgramAccount(buf2, MEMORY_BUFFER_HEADER + 32);
      await program.methods
        .initBuffer(AGENT_ID, 32)
        .accounts({ buffer: buf2.publicKey })
        .rpc();

      const agent = await program.account.agentState.fetch(agentPDA(AGENT_ID));
      expect(agent.pendingBuffer.toBase58()).to.eq(buf2.publicKey.toBase58());

      // Cleanup
      await program.methods
        .closeBuffer(AGENT_ID)
        .accounts({ buffer: buf2.publicKey })
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      // Write only half — buffer remains incomplete
      await program.methods
        .writeToBuffer(AGENT_ID, 0, Buffer.alloc(10))
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
    });

    it("rejects finalize when buffer is not fully written (BufferIncomplete)", async () => {
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + TOTAL_LEN);
      try {
        await program.methods
          .finalizeMemoryNew(AGENT_ID)
          .accounts({
            buffer: bufKp.publicKey,
            newMemory: memoryKp.publicKey,
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
        .accounts({ buffer: bufKp.publicKey })
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, V1)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await createProgramAccount(memoryV1Kp, AGENT_MEMORY_HEADER + V1.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accounts({ buffer: bufKp.publicKey, newMemory: memoryV1Kp.publicKey })
        .rpc();
    });

    it("replaces memory and closes old memory account", async () => {
      const bufV2Kp = Keypair.generate();
      const memoryV2Kp = Keypair.generate();

      await createProgramAccount(bufV2Kp, MEMORY_BUFFER_HEADER + V2.length);
      await program.methods
        .initBuffer(AGENT_ID, V2.length)
        .accounts({ buffer: bufV2Kp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, V2)
        .accounts({ buffer: bufV2Kp.publicKey })
        .rpc();
      await createProgramAccount(memoryV2Kp, AGENT_MEMORY_HEADER + V2.length);

      await program.methods
        .finalizeMemoryUpdate(AGENT_ID)
        .accounts({ buffer: bufV2Kp.publicKey, newMemory: memoryV2Kp.publicKey, oldMemory: memoryV1Kp.publicKey })
        .rpc();

      const agent = await program.account.agentState.fetch(agentPDA(AGENT_ID));
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, tiny)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + tiny.length);

      try {
        await program.methods
          .finalizeMemoryNew(AGENT_ID)
          .accounts({
            buffer: bufKp.publicKey,
            newMemory: memoryKp.publicKey,
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
        .accounts({ buffer: bufKp.publicKey })
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
        .accounts({ buffer: bufKp2.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(emptyId, 0, data)
        .accounts({ buffer: bufKp2.publicKey })
        .rpc();
      await createProgramAccount(memoryKp2, AGENT_MEMORY_HEADER + data.length);
      await createProgramAccount(dummyOldMemory, AGENT_MEMORY_HEADER + data.length);

      try {
        await program.methods
          .finalizeMemoryUpdate(emptyId)
          .accounts({ buffer: bufKp2.publicKey, newMemory: memoryKp2.publicKey, oldMemory: dummyOldMemory.publicKey })
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
        .accounts({ buffer: bufKp2.publicKey })
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, INITIAL)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + INITIAL.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accounts({ buffer: bufKp.publicKey, newMemory: memoryKp.publicKey })
        .rpc();
    });

    it("appends buffer data to existing memory without allocating new account", async () => {
      const agentKey = agentPDA(AGENT_ID);
      const appendBufKp = Keypair.generate();

      await createProgramAccount(appendBufKp, MEMORY_BUFFER_HEADER + APPEND.length);
      await program.methods
        .initBuffer(AGENT_ID, APPEND.length)
        .accounts({ buffer: appendBufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, APPEND)
        .accounts({ buffer: appendBufKp.publicKey })
        .rpc();

      await program.methods
        .finalizeMemoryAppend(AGENT_ID)
        .accounts({ buffer: appendBufKp.publicKey, memory: memoryKp.publicKey })
        .rpc();

      // AgentState updated.
      const agent = await program.account.agentState.fetch(agentKey);
      expect(agent.memory.toBase58()).to.eq(memoryKp.publicKey.toBase58());
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(emptyId, 0, data)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();

      try {
        await program.methods
          .finalizeMemoryAppend(emptyId)
          .accounts({ buffer: bufKp.publicKey, memory: authority.publicKey })
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
    });
  });

  // ── set_referral ────────────────────────────────────────────────────────
  describe("set_referral", () => {
    const AGENT_ID = "set-ref-agent";
    const REFERRAL_ID = "set-ref-referral";

    before(async () => {
      await doRegisterAgent(AGENT_ID);
      await doRegisterAgent(REFERRAL_ID);
    });

    it("sets referral on an agent without one", async () => {
      const refereeAta = getAssociatedTokenAddressSync(refereeMintPDA(), authority.publicKey, false, TOKEN_2022_PROGRAM_ID);
      await program.methods
        .setReferral(AGENT_ID)
        .accounts({
          referralAgent: agentPDA(REFERRAL_ID),
          referralAuthority: authority.publicKey,
          referralRefereeAccount: refereeAta,
        })
        .rpc();

      const agent = await program.account.agentState.fetch(agentPDA(AGENT_ID));
      const ridLen = agent.referralIdLen;
      const ridBytes = agent.referralId.slice(0, ridLen);
      const rid = Buffer.from(ridBytes).toString("utf-8");
      expect(rid).to.eq(REFERRAL_ID);

      // Verify NARA Referee token was minted (1 from set_referral)
      const balance = await getRefereeBalance(authority.publicKey);
      expect(balance).to.eq(BigInt(1));
    });

    it("rejects setting referral again (ReferralAlreadySet)", async () => {
      const refereeAta = getAssociatedTokenAddressSync(refereeMintPDA(), authority.publicKey, false, TOKEN_2022_PROGRAM_ID);
      try {
        await program.methods
          .setReferral(AGENT_ID)
          .accounts({
            referralAgent: agentPDA(REFERRAL_ID),
            referralAuthority: authority.publicKey,
            referralRefereeAccount: refereeAta,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.error?.errorCode?.code).to.eq("ReferralAlreadySet");
      }
    });

    it("rejects self-referral", async () => {
      const SELF_AGENT = "self-ref-agent";
      await doRegisterAgent(SELF_AGENT);
      const refereeAta = getAssociatedTokenAddressSync(refereeMintPDA(), authority.publicKey, false, TOKEN_2022_PROGRAM_ID);
      try {
        await program.methods
          .setReferral(SELF_AGENT)
          .accounts({
            referralAgent: agentPDA(SELF_AGENT),
            referralAuthority: authority.publicKey,
            referralRefereeAccount: refereeAta,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.error?.errorCode?.code).to.eq("SelfReferral");
      }
    });

    it("rejects non-authority signer", async () => {
      const NOAUTH_AGENT = "noauth-ref-ag";
      await doRegisterAgent(NOAUTH_AGENT);
      const fakeAuth = Keypair.generate();
      const refereeAta = getAssociatedTokenAddressSync(refereeMintPDA(), authority.publicKey, false, TOKEN_2022_PROGRAM_ID);
      try {
        await program.methods
          .setReferral(NOAUTH_AGENT)
          .accounts({
            authority: fakeAuth.publicKey,
            referralAgent: agentPDA(REFERRAL_ID),
            referralAuthority: authority.publicKey,
            referralRefereeAccount: refereeAta,
          } as any)
          .signers([fakeAuth])
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.toString()).to.include("Unauthorized");
      }
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
        .accounts({
          authority: authority.publicKey,
          agent: agentPDA(AGENT_ID),
          bioAccount: bioPDA(agentPDA(AGENT_ID)),
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods
        .setMetadata(AGENT_ID, JSON.stringify({ tag: "temp" }))
        .accounts({
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await program.methods
        .writeToBuffer(AGENT_ID, 0, CONTENT)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
      await createProgramAccount(memoryKp, AGENT_MEMORY_HEADER + CONTENT.length);
      await program.methods
        .finalizeMemoryNew(AGENT_ID)
        .accounts({ buffer: bufKp.publicKey, newMemory: memoryKp.publicKey })
        .rpc();
    });

    it("closes agent state, bio, metadata, and memory; returns rent", async () => {
      const agentKey = agentPDA(AGENT_ID);

      await program.methods
        .deleteAgent(AGENT_ID)
        .accounts({ memoryAccount: memoryKp.publicKey })
        .rpc();

      expect(await provider.connection.getAccountInfo(agentKey)).to.be.null;
      expect(await provider.connection.getAccountInfo(bioPDA(agentKey))).to.be.null;
      expect(await provider.connection.getAccountInfo(metaPDA(agentKey))).to.be.null;
      expect(await provider.connection.getAccountInfo(memoryKp.publicKey)).to.be.null;
    });

    it("allows re-registration with the same agent_id after deletion", async () => {
      const agentKey = agentPDA(AGENT_ID);
      await doRegisterAgent(AGENT_ID);

      const agent = await program.account.agentState.fetch(agentKey);
      expect(parseAgentId(agent)).to.eq(AGENT_ID);
      expect(agent.version).to.eq(0);
    });

    it("rejects non-authority (Unauthorized)", async () => {
      const other = Keypair.generate();
      const agentKey = agentPDA(AGENT_ID);
      try {
        await program.methods
          .deleteAgent(AGENT_ID)
          .accounts({ authority: other.publicKey, memoryAccount: authority.publicKey } as any)
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
        .accounts({ buffer: bufKp.publicKey })
        .rpc();

      try {
        await program.methods
          .deleteAgent(agentId3)
          .accounts({ memoryAccount: authority.publicKey })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("HasPendingBuffer");
      }

      // Cleanup
      await program.methods
        .closeBuffer(agentId3)
        .accounts({ buffer: bufKp.publicKey })
        .rpc();
    });
  });

  // ── log_activity (no referral) ────────────────────────────────────────────
  describe("log_activity", () => {
    const AGENT_ID = "log-agent-01";

    before(async () => {
      await doRegisterAgent(AGENT_ID);
    });

    it("emits ActivityLogged event (no quest ix → no points minted)", async () => {
      const listener = program.addEventListener("activityLogged", (event) => {
        expect(event.agentId).to.eq(AGENT_ID);
        expect(event.model).to.eq("gpt-4");
        expect(event.activity).to.eq("chat");
        expect(event.log).to.eq("handled user query about weather");
        expect(event.referralId).to.eq("");
        expect(event.pointsEarned.toNumber()).to.eq(0);
        expect(event.referralPointsEarned.toNumber()).to.eq(0);
        expect(event.authority.toBase58()).to.eq(authority.publicKey.toBase58());
        expect(event.timestamp.toNumber()).to.be.greaterThan(0);
      });

      await program.methods
        .logActivity(AGENT_ID, "gpt-4", "chat", "handled user query about weather")
        .accounts({})
        .rpc();

      await new Promise((resolve) => setTimeout(resolve, 2000));
      program.removeEventListener(listener);
    });

    it("rejects non-authority signer", async () => {
      const other = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(other.publicKey, web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
      const mint = pointMintPDA();
      const otherAta = getAssociatedTokenAddressSync(mint, other.publicKey, false, TOKEN_2022_PROGRAM_ID);
      try {
        await program.methods
          .logActivity(AGENT_ID, "gpt-4", "chat", "evil log")
          .accounts({
            authority: other.publicKey,
            authorityPointAccount: otherAta,
          } as any)
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });
  });

  // ── log_activity_with_referral ──────────────────────────────────────────
  describe("log_activity_with_referral", () => {
    const AGENT_ID = "log-ref-agent";
    const REFERRAL_ID = "log-ref-referral";

    before(async () => {
      await doRegisterAgent(REFERRAL_ID);
      await doRegisterAgentWithReferral(
        AGENT_ID,
        agentPDA(REFERRAL_ID),
        authority.publicKey,
      );
    });

    it("emits ActivityLogged event with referral (no quest ix → no points minted)", async () => {
      const referralKey = agentPDA(REFERRAL_ID);
      const listener = program.addEventListener("activityLogged", (event) => {
        expect(event.agentId).to.eq(AGENT_ID);
        expect(event.model).to.eq("gpt-4");
        expect(event.activity).to.eq("chat");
        expect(event.log).to.eq("handled user query about weather");
        expect(event.referralId).to.eq(REFERRAL_ID);
        expect(event.pointsEarned.toNumber()).to.eq(0);
        expect(event.referralPointsEarned.toNumber()).to.eq(0);
      });

      await program.methods
        .logActivityWithReferral(AGENT_ID, "gpt-4", "chat", "handled user query about weather")
        .accounts({
          referralAgent: referralKey,
          referralAuthority: authority.publicKey,
        })
        .rpc();

      await new Promise((resolve) => setTimeout(resolve, 2000));
      program.removeEventListener(listener);

      // No quest ix in this tx, so no points should be minted
      // But register_agent with referral already minted 10 referral_register_points
      const balance = await getPointBalance(authority.publicKey);
      expect(balance).to.eq(BigInt(10));
    });

    it("agent with referral can also use logActivity (no referral version)", async () => {
      await program.methods
        .logActivity(AGENT_ID, "gpt-4", "chat", "using simple log")
        .accounts({})
        .rpc();

      // Still 10 from registration, no new minting
      const balance = await getPointBalance(authority.publicKey);
      expect(balance).to.eq(BigInt(10));
    });

    it("rejects non-authority signer", async () => {
      const referralKey = agentPDA(REFERRAL_ID);
      const other = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(other.publicKey, web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
      const mint = pointMintPDA();
      const otherAta = getAssociatedTokenAddressSync(mint, other.publicKey, false, TOKEN_2022_PROGRAM_ID);
      try {
        await program.methods
          .logActivityWithReferral(AGENT_ID, "gpt-4", "chat", "evil log")
          .accounts({
            authority: other.publicKey,
            authorityPointAccount: otherAta,
            referralAgent: referralKey,
            referralAuthority: authority.publicKey,
          } as any)
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });
  });

  // ── twitter_verification ─────────────────────────────────────────────────
  describe("twitter_verification", () => {
    const TWITTER_AGENT_ID = "twitter-agent-01";
    const TWITTER_USERNAME = "naraproject";
    const TWEET_URL = "https://x.com/naraproject/status/123456789";
    const VERIFY_FEE = new anchor.BN(10_000_000); // 0.01 SOL for testing
    let verifier: Keypair;

    before(async () => {
      // Register the test agent (fee=0 to avoid needing SOL for registration)
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(TWITTER_AGENT_ID);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      // Set up verifier keypair
      verifier = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(verifier.publicKey, 5 * web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);

      // Admin sets the twitter verifier
      await program.methods.updateTwitterVerifier(verifier.publicKey).accounts({}).rpc();

      // Admin sets verification fee
      await program.methods
        .updateTwitterVerificationConfig(VERIFY_FEE, new anchor.BN(0), new anchor.BN(0))
        .accounts({})
        .rpc();
    });

    it("update_twitter_verifier: admin sets verifier address", async () => {
      const cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.twitterVerifier.toBase58()).to.eq(verifier.publicKey.toBase58());
    });

    it("update_twitter_verification_config: admin sets fee", async () => {
      const cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.twitterVerificationFee.toNumber()).to.eq(VERIFY_FEE.toNumber());
    });

    it("set_twitter: agent sets username and tweet_url, pays fee", async () => {
      const vaultKey = twitterVerifyVaultPDA();
      const vaultBefore = await provider.connection.getBalance(vaultKey);

      await program.methods
        .setTwitter(TWITTER_AGENT_ID, TWITTER_USERNAME, TWEET_URL)
        .accounts({
          twitterVerifyVault: vaultKey,
        })
        .rpc();

      const vaultAfter = await provider.connection.getBalance(vaultKey);
      expect(vaultAfter - vaultBefore).to.eq(VERIFY_FEE.toNumber());

      // Read the AgentTwitter account
      const agentKey = agentPDA(TWITTER_AGENT_ID);
      const twitterKey = twitterPDA(agentKey);
      const twitterAcc = await program.account.agentTwitter.fetch(twitterKey);
      expect(twitterAcc.status.toNumber()).to.eq(1); // Pending
      expect(Buffer.from(twitterAcc.username.slice(0, Number(twitterAcc.usernameLen))).toString()).to.eq(TWITTER_USERNAME);
      expect(Buffer.from(twitterAcc.tweetUrl.slice(0, Number(twitterAcc.tweetUrlLen))).toString()).to.eq(TWEET_URL);

      // Queue should contain the AgentTwitter PDA
      const queue = await readTwitterQueue();
      expect(queue.some(k => k.equals(twitterKey))).to.be.true;
    });

    it("set_twitter: rejects non-authority", async () => {
      const other = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(other.publicKey, web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);

      try {
        await program.methods
          .setTwitter(TWITTER_AGENT_ID, "other_user", "https://x.com/test/status/1")
          .accounts({
            authority: other.publicKey,
            twitterVerifyVault: twitterVerifyVaultPDA(),
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("Unauthorized");
      }
    });

    it("set_twitter: rejects empty username", async () => {
      try {
        await program.methods
          .setTwitter(TWITTER_AGENT_ID, "", TWEET_URL)
          .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("TwitterUsernameEmpty");
      }
    });

    it("verify_twitter: verifier approves, fee refunded, TwitterHandle created", async () => {
      const agentKey = agentPDA(TWITTER_AGENT_ID);
      const authorityBefore = await provider.connection.getBalance(authority.publicKey);

      await program.methods
        .verifyTwitter(TWITTER_AGENT_ID, TWITTER_USERNAME)
        .accounts({
          verifier: verifier.publicKey,
          authority: authority.publicKey,
          twitterVerifyVault: twitterVerifyVaultPDA(),
          treasury: treasuryPDA(),
        })
        .signers([verifier])
        .rpc();

      // Check status is Verified
      const twitterKey = twitterPDA(agentKey);
      const twitterAcc = await program.account.agentTwitter.fetch(twitterKey);
      expect(twitterAcc.status.toNumber()).to.eq(2); // Verified
      expect(twitterAcc.verifiedAt.toNumber()).to.be.greaterThan(0);

      // Check TwitterHandle was created
      const handleKey = twitterHandlePDA(TWITTER_USERNAME);
      const handleAcc = await program.account.twitterHandle.fetch(handleKey);
      expect(handleAcc.agent.toBase58()).to.eq(agentKey.toBase58());

      // Check fee was refunded to authority
      const authorityAfter = await provider.connection.getBalance(authority.publicKey);
      expect(authorityAfter).to.be.greaterThan(authorityBefore - 100_000); // minus possible tx fees for other txs

      // Queue should no longer contain this twitter PDA
      const queue = await readTwitterQueue();
      expect(queue.some(k => k.equals(twitterKey))).to.be.false;
    });

    it("verify_twitter: rejects non-verifier", async () => {
      // Need a new agent in pending state for this test
      const newAgentId = "twitter-agent-02";
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(newAgentId);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      await program.methods
        .setTwitter(newAgentId, "other_handle", "https://x.com/test/status/999")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      // Queue should contain agent-02's twitter PDA (queue length 1 after agent-01 was verified)
      const agent02Twitter = twitterPDA(agentPDA(newAgentId));
      const queueAfter02 = await readTwitterQueue();
      expect(queueAfter02.length).to.eq(1);
      expect(queueAfter02.some(k => k.equals(agent02Twitter))).to.be.true;

      const other = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(other.publicKey, web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);

      try {
        await program.methods
          .verifyTwitter(newAgentId, "other_handle")
          .accounts({
            verifier: other.publicKey,
            authority: authority.publicKey,
            twitterVerifyVault: twitterVerifyVaultPDA(),
            treasury: treasuryPDA(),
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("NotTwitterVerifier");
      }
    });

    it("verify_twitter: rejects duplicate twitter handle", async () => {
      // twitter-agent-02 is pending with "other_handle", but let's try with TWITTER_USERNAME which is already verified
      const newAgentId = "twitter-agent-03";
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(newAgentId);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      await program.methods
        .setTwitter(newAgentId, TWITTER_USERNAME, "https://x.com/test/status/dup")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      // Queue should now have 2 entries: agent-02 and agent-03
      const agent02Twitter = twitterPDA(agentPDA("twitter-agent-02"));
      const agent03Twitter = twitterPDA(agentPDA(newAgentId));
      const queueAfter03 = await readTwitterQueue();
      expect(queueAfter03.length).to.eq(2);
      expect(queueAfter03.some(k => k.equals(agent02Twitter))).to.be.true;
      expect(queueAfter03.some(k => k.equals(agent03Twitter))).to.be.true;

      try {
        await program.methods
          .verifyTwitter(newAgentId, TWITTER_USERNAME)
          .accounts({
            verifier: verifier.publicKey,
            authority: authority.publicKey,
            twitterVerifyVault: twitterVerifyVaultPDA(),
            treasury: treasuryPDA(),
          })
          .signers([verifier])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        // TwitterHandle PDA already exists, init will fail
        expect(e.toString()).to.include("already in use");
      }
    });

    it("reject_twitter: verifier rejects, fee not refunded", async () => {
      // Use twitter-agent-02 which is still pending
      const agentId = "twitter-agent-02";
      const agentKey = agentPDA(agentId);

      await program.methods
        .rejectTwitter(agentId)
        .accounts({
          verifier: verifier.publicKey,
        })
        .signers([verifier])
        .rpc();

      const twitterKey = twitterPDA(agentKey);
      const twitterAcc = await program.account.agentTwitter.fetch(twitterKey);
      expect(twitterAcc.status.toNumber()).to.eq(3); // Rejected

      // agent-02 was at index 0, agent-03 at index 1 (len=2).
      // swap-and-pop: agent-03 moves to index 0, len becomes 1.
      const agent03Twitter = twitterPDA(agentPDA("twitter-agent-03"));
      const queue = await readTwitterQueue();
      expect(queue.length).to.eq(1, "queue should have 1 entry after rejecting agent-02");
      expect(queue.some(k => k.equals(twitterKey))).to.be.false;    // agent-02 gone
      expect(queue.some(k => k.equals(agent03Twitter))).to.be.true; // agent-03 survived swap
    });

    it("unbind_twitter: agent unbinds verified twitter, pays fee, PDAs closed", async () => {
      const agentKey = agentPDA(TWITTER_AGENT_ID);
      const twitterKey = twitterPDA(agentKey);
      const handleKey = twitterHandlePDA(TWITTER_USERNAME);

      // Verify accounts exist before unbind
      const twitterBefore = await provider.connection.getAccountInfo(twitterKey);
      expect(twitterBefore).to.not.be.null;
      const handleBefore = await provider.connection.getAccountInfo(handleKey);
      expect(handleBefore).to.not.be.null;

      const vaultBefore = await provider.connection.getBalance(twitterVerifyVaultPDA());

      await program.methods
        .unbindTwitter(TWITTER_AGENT_ID, TWITTER_USERNAME)
        .accounts({
          twitterVerifyVault: twitterVerifyVaultPDA(),
        })
        .rpc();

      // Both accounts should be closed
      const twitterAfter = await provider.connection.getAccountInfo(twitterKey);
      expect(twitterAfter).to.be.null;
      const handleAfter = await provider.connection.getAccountInfo(handleKey);
      expect(handleAfter).to.be.null;

      // Unbind fee went to vault
      const vaultAfter = await provider.connection.getBalance(twitterVerifyVaultPDA());
      expect(vaultAfter - vaultBefore).to.eq(1_000_000_000); // 1 NARA
    });

    it("unbind_twitter: after unbind, same username can be re-bound", async () => {
      // Re-set twitter on the same agent (account was closed, so init again)
      await program.methods
        .setTwitter(TWITTER_AGENT_ID, TWITTER_USERNAME, "https://x.com/naraproject/status/newtweet")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      const agentKey = agentPDA(TWITTER_AGENT_ID);
      const twitterKey = twitterPDA(agentKey);
      const twitterAcc = await program.account.agentTwitter.fetch(twitterKey);
      expect(twitterAcc.status.toNumber()).to.eq(1); // Pending again
    });

    it("withdraw_twitter_verify_fees: admin withdraws", async () => {
      const vaultKey = twitterVerifyVaultPDA();
      const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(0);
      const vaultBalance = await provider.connection.getBalance(vaultKey);
      const available = vaultBalance - rentExempt;

      if (available > 0) {
        await program.methods
          .withdrawTwitterVerifyFees(new anchor.BN(available))
          .accounts({})
          .rpc();

        const vaultAfter = await provider.connection.getBalance(vaultKey);
        expect(vaultAfter).to.eq(rentExempt);
      }
    });
  });

  describe("tweet_verification", () => {
    const TWEET_AGENT_ID = "tweet-verify-agent";
    const TWEET_USERNAME = "tweetverifier";
    const TWEET_URL = "https://x.com/tweetverifier/status/111222333";
    const VERIFY_FEE = new anchor.BN(5_000_000); // 0.005 SOL
    const TWEET_REWARD = new anchor.BN(10_000_000); // 0.01 SOL
    const TWEET_POINTS = new anchor.BN(5);
    let verifier: Keypair;

    before(async () => {
      // Register a new agent
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(TWEET_AGENT_ID);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      // Set up verifier
      verifier = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(verifier.publicKey, 10 * web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);
      await program.methods.updateTwitterVerifier(verifier.publicKey).accounts({}).rpc();

      // Set verification fee
      await program.methods
        .updateTwitterVerificationConfig(VERIFY_FEE, new anchor.BN(0), new anchor.BN(0))
        .accounts({})
        .rpc();

      // Set tweet verify reward config
      await program.methods
        .updateTweetVerifyConfig(TWEET_REWARD, TWEET_POINTS)
        .accounts({})
        .rpc();

      // Fund treasury for rewards
      const treasuryKey = treasuryPDA();
      const treasurySig = await provider.connection.requestAirdrop(treasuryKey, 2 * web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(treasurySig);

      // Bind and verify twitter for the agent
      const agentKey = agentPDA(TWEET_AGENT_ID);
      await program.methods
        .setTwitter(TWEET_AGENT_ID, TWEET_USERNAME, "https://x.com/tweetverifier/status/bind")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      await program.methods
        .verifyTwitter(TWEET_AGENT_ID, TWEET_USERNAME)
        .accounts({
          verifier: verifier.publicKey,
          authority: authority.publicKey,
          twitterVerifyVault: twitterVerifyVaultPDA(),
          treasury: treasuryPDA(),
        })
        .signers([verifier])
        .rpc();

      // Confirm twitter is verified
      const twitterAcc = await program.account.agentTwitter.fetch(twitterPDA(agentKey));
      expect(twitterAcc.status.toNumber()).to.eq(2);
    });

    it("update_tweet_verify_config: admin sets reward and points", async () => {
      const cfg = await program.account.programConfig.fetch(configPDA());
      expect(cfg.tweetVerifyReward.toNumber()).to.eq(TWEET_REWARD.toNumber());
      expect(cfg.tweetVerifyPoints.toNumber()).to.eq(TWEET_POINTS.toNumber());
    });

    it("submit_tweet: submits tweet, pays fee, TweetVerify created", async () => {
      const agentKey = agentPDA(TWEET_AGENT_ID);
      const vaultBefore = await provider.connection.getBalance(twitterVerifyVaultPDA());

      await program.methods
        .submitTweet(TWEET_AGENT_ID, TWEET_URL)
        .accounts({
          twitterVerifyVault: twitterVerifyVaultPDA(),
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .rpc();

      // Fee paid to vault
      const vaultAfter = await provider.connection.getBalance(twitterVerifyVaultPDA());
      expect(vaultAfter - vaultBefore).to.eq(VERIFY_FEE.toNumber());

      // TweetVerify PDA created with Pending status
      const tvKey = tweetVerifyPDA(agentKey);
      const tvAcc = await program.account.tweetVerify.fetch(tvKey);
      expect(tvAcc.status.toNumber()).to.eq(1); // Pending
      expect(tvAcc.submittedAt.toNumber()).to.be.greaterThan(0);
      expect(
        Buffer.from(tvAcc.tweetUrl.slice(0, Number(tvAcc.tweetUrlLen))).toString()
      ).to.eq(TWEET_URL);

      // Queue should contain TweetVerify PDA
      const queue = await readTweetVerifyQueue();
      expect(queue.some(k => k.equals(tvKey))).to.be.true;
    });

    it("submit_tweet: rejects when already pending", async () => {
      try {
        await program.methods
          .submitTweet(TWEET_AGENT_ID, "https://x.com/test/status/2")
          .accounts({
            twitterVerifyVault: twitterVerifyVaultPDA(),
            tweetVerifyQueue: tweetVerifyQueuePDA(),
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("TweetVerifyAlreadyPending");
      }
    });

    it("submit_tweet: setup tweet-verify-agent-2 with verified twitter", async () => {
      // Register another agent and verify twitter for later tests
      const otherAgentId = "tweet-verify-agent-2";
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(otherAgentId);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      const otherUsername = "othertwitter";
      await program.methods
        .setTwitter(otherAgentId, otherUsername, "https://x.com/other/status/bind")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      await program.methods
        .verifyTwitter(otherAgentId, otherUsername)
        .accounts({
          verifier: verifier.publicKey,
          authority: authority.publicKey,
          twitterVerifyVault: twitterVerifyVaultPDA(),
          treasury: treasuryPDA(),
        })
        .signers([verifier])
        .rpc();
    });

    it("approve_tweet: verifier approves, fee refunded, rewards issued", async () => {
      const agentKey = agentPDA(TWEET_AGENT_ID);
      const authorityBefore = await provider.connection.getBalance(authority.publicKey);
      const pointsBefore = await getPointBalance(authority.publicKey);

      await program.methods
        .approveTweet(TWEET_AGENT_ID)
        .accounts({
          verifier: verifier.publicKey,
          authority: authority.publicKey,
          twitterVerifyVault: twitterVerifyVaultPDA(),
          treasury: treasuryPDA(),
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .signers([verifier])
        .rpc();

      // TweetVerify: status=Idle, last_rewarded_at set
      const tvAcc = await program.account.tweetVerify.fetch(tweetVerifyPDA(agentKey));
      expect(tvAcc.status.toNumber()).to.eq(0); // Idle
      expect(tvAcc.lastRewardedAt.toNumber()).to.be.greaterThan(0);

      // Points minted
      const pointsAfter = await getPointBalance(authority.publicKey);
      expect(Number(pointsAfter - pointsBefore)).to.eq(TWEET_POINTS.toNumber());

      // Queue empty
      const queue = await readTweetVerifyQueue();
      expect(queue.some(k => k.equals(tweetVerifyPDA(agentKey)))).to.be.false;
    });

    it("approve_tweet: rejects non-verifier", async () => {
      // First submit a new tweet
      await program.methods
        .submitTweet("tweet-verify-agent-2", "https://x.com/test/status/4")
        .accounts({
          twitterVerifyVault: twitterVerifyVaultPDA(),
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .rpc();

      const other = Keypair.generate();
      const sig = await provider.connection.requestAirdrop(other.publicKey, web3.LAMPORTS_PER_SOL);
      await provider.connection.confirmTransaction(sig);

      try {
        await program.methods
          .approveTweet("tweet-verify-agent-2")
          .accounts({
            verifier: other.publicKey,
            authority: authority.publicKey,
            twitterVerifyVault: twitterVerifyVaultPDA(),
            treasury: treasuryPDA(),
            tweetVerifyQueue: tweetVerifyQueuePDA(),
          })
          .signers([other])
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("NotTwitterVerifier");
      }
    });

    it("reject_tweet: verifier rejects, fee not refunded, status back to Idle", async () => {
      // tweet-verify-agent-2 has a pending tweet from previous test
      const agentKey = agentPDA("tweet-verify-agent-2");
      const vaultBefore = await provider.connection.getBalance(twitterVerifyVaultPDA());

      await program.methods
        .rejectTweet("tweet-verify-agent-2")
        .accounts({
          verifier: verifier.publicKey,
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .signers([verifier])
        .rpc();

      const tvAcc = await program.account.tweetVerify.fetch(tweetVerifyPDA(agentKey));
      expect(tvAcc.status.toNumber()).to.eq(0); // Idle

      // Vault balance unchanged (fee not refunded)
      const vaultAfter = await provider.connection.getBalance(twitterVerifyVaultPDA());
      expect(vaultAfter).to.eq(vaultBefore);

      // Queue should be empty
      const queue = await readTweetVerifyQueue();
      expect(queue.length).to.eq(0);
    });

    it("reject_tweet then resubmit: can resubmit immediately after rejection", async () => {
      // tweet-verify-agent-2 was rejected, should be able to resubmit
      await program.methods
        .submitTweet("tweet-verify-agent-2", "https://x.com/test/status/5")
        .accounts({
          twitterVerifyVault: twitterVerifyVaultPDA(),
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .rpc();

      const agentKey = agentPDA("tweet-verify-agent-2");
      const tvAcc = await program.account.tweetVerify.fetch(tweetVerifyPDA(agentKey));
      expect(tvAcc.status.toNumber()).to.eq(1); // Pending again

      // Clean up: reject so it doesn't interfere
      await program.methods
        .rejectTweet("tweet-verify-agent-2")
        .accounts({
          verifier: verifier.publicKey,
          tweetVerifyQueue: tweetVerifyQueuePDA(),
        })
        .signers([verifier])
        .rpc();
    });

    it("submit_tweet: rejects during cooldown period", async () => {
      // tweet-verify-agent was approved earlier, so last_rewarded_at is recent
      // Cooldown is 24 hours, so submitting again should fail
      try {
        await program.methods
          .submitTweet(TWEET_AGENT_ID, "https://x.com/test/status/cooldown")
          .accounts({
            twitterVerifyVault: twitterVerifyVaultPDA(),
            tweetVerifyQueue: tweetVerifyQueuePDA(),
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("TweetVerifyCooldown");
      }
    });

    it("submit_tweet: rejects when twitter not verified", async () => {
      // Register a new agent without verified twitter
      const noTwitterAgent = "no-twitter-agent";
      await program.methods.updateRegisterFee(new anchor.BN(0)).accounts({}).rpc();
      await doRegisterAgent(noTwitterAgent);
      await program.methods.updateRegisterFee(ONE_SOL).accounts({}).rpc();

      // Set twitter but don't verify — status=Pending(1), not Verified(2)
      await program.methods
        .setTwitter(noTwitterAgent, "unverified_user", "https://x.com/test/status/noverify")
        .accounts({ twitterVerifyVault: twitterVerifyVaultPDA() })
        .rpc();

      try {
        await program.methods
          .submitTweet(noTwitterAgent, "https://x.com/test/status/6")
          .accounts({
            twitterVerifyVault: twitterVerifyVaultPDA(),
            tweetVerifyQueue: tweetVerifyQueuePDA(),
          })
          .rpc();
        expect.fail("expected error");
      } catch (e: any) {
        expect(e.error?.errorCode?.code ?? e.message).to.include("TwitterNotVerified");
      }
    });
  });
});
