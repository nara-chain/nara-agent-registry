/**
 * Standalone initialization script for Nara Agent Registry.
 *
 * Usage:
 *   TEST_RPC_URL=http://127.0.0.1:8899 TEST_PRIVATE_KEY=<base58> tsx tests/init.ts
 *
 * Calls init_config() once. If config already exists, skips gracefully.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import fs from "fs";
import path from "path";
// bs58 v4 ships no type declarations; require + cast avoids the hint
// eslint-disable-next-line @typescript-eslint/no-require-imports
const bs58 = require("bs58") as { decode: (input: string) => Buffer };
import { NaraAgentRegistry } from "../target/types/nara_agent_registry";

// ── Config from environment ──────────────────────────────────────────────────
const CLUSTER = process.env.TEST_RPC_URL ?? "http://127.0.0.1:8899";
const PRIVATE_KEY = process.env.TEST_PRIVATE_KEY;
if (!PRIVATE_KEY) {
  console.error("Error: TEST_PRIVATE_KEY is not set");
  process.exit(1);
}

// ── Load wallet ──────────────────────────────────────────────────────────────
const adminKeypair = web3.Keypair.fromSecretKey(bs58.decode(PRIVATE_KEY));
const wallet = new anchor.Wallet(adminKeypair);

// ── Provider + program ───────────────────────────────────────────────────────
const connection = new web3.Connection(CLUSTER, "confirmed");
const provider = new anchor.AnchorProvider(connection, wallet, {
  commitment: "confirmed",
});
anchor.setProvider(provider);

const idlPath = path.join(__dirname, "../target/idl/nara_agent_registry.json");
const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
const program = new Program<NaraAgentRegistry>(idl, provider);

// ── PDA helpers ───────────────────────────────────────────────────────────────
const configPDA = (): PublicKey =>
  PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    program.programId
  )[0];

// ── Main ──────────────────────────────────────────────────────────────────────
async function main() {
  console.log("Cluster  :", CLUSTER);
  console.log("Admin    :", adminKeypair.publicKey.toBase58());
  console.log("Program  :", program.programId.toBase58());

  const configKey = configPDA();
  console.log("Config   :", configKey.toBase58());

  // Check if already initialized
  const existing = await connection.getAccountInfo(configKey);
  if (existing !== null) {
    const cfg = await program.account.programConfig.fetch(configKey);
    console.log("\nConfig already initialized:");
    console.log("  admin         :", cfg.admin.toBase58());
    console.log("  registerFee   :", cfg.registerFee.toString(), "lamports");
    console.log("  feeVault      :", cfg.feeVault.toBase58());
    return;
  }

  // Initialize
  console.log("\nInitializing config...");
  const tx = await program.methods
    .initConfig()
    .accounts({})
    .rpc();
  console.log("Tx:", tx);

  const cfg = await program.account.programConfig.fetch(configKey);
  console.log("\nConfig initialized:");
  console.log("  admin         :", cfg.admin.toBase58());
  console.log("  registerFee   :", cfg.registerFee.toString(), "lamports");
  console.log("  feeVault      :", cfg.feeVault.toBase58());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
