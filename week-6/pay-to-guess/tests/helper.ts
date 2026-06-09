import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import fs from "fs";
import { PayToGuess } from "../target/types/pay_to_guess";
import { Program } from "@coral-xyz/anchor";
import { randomBytes } from "crypto";
import { expect } from "chai";

export const DEFAULT_QUEUE = new PublicKey(
  "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc"
);
export const ER_URL = "https://devnet-as.magicblock.app/";

export async function fundAccount(
  connection: Connection,
  payer: Keypair,
  recipient: PublicKey,
  sol: number
): Promise<string> {
  const tx = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: recipient,
      lamports: sol * LAMPORTS_PER_SOL,
    })
  );

  return await sendAndConfirmTransaction(connection, tx, [payer], {
    commitment: "confirmed",
  });
}

// Load player from file
export function loadPlayer(filePath: string): Keypair {
  const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(data));
}

const GAME_SEED = Buffer.from("game");
const VAULT_SEED = Buffer.from("game_vault");
const PLAYER_SEED = Buffer.from("player_state");

export function getGamePDA(authority: PublicKey, programId: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [GAME_SEED, authority.toBuffer()],
    programId
  );
}
export function getVaultPDA(gamePDA: PublicKey, programId: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, gamePDA.toBuffer()],
    programId
  );
}
export function getPlayerStatePDA(player: PublicKey, programId: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [PLAYER_SEED, player.toBuffer()],
    programId
  );
}

export async function buildPlayTx(
  program: Program<PayToGuess>,
  player: Keypair,
  gamePDA: PublicKey,
  vaultPDA: PublicKey,
  playerStatePDA: PublicKey,
  lamports: number,
  guess: number
): Promise<Transaction> {
  const tx = new Transaction();

  // ix[0]: pay the vault — introspection will verify this
  tx.add(
    SystemProgram.transfer({
      fromPubkey: player.publicKey,
      toPubkey: vaultPDA,
      lamports,
    })
  );

  // ix[1]: submit guess — introspection checks ix[0] at runtime
  tx.add(
    await program.methods
      .play({ guess })
      .accounts({
        player: player.publicKey,
        //@ts-ignore
        playerState: playerStatePDA,
        game: gamePDA,
        gameVault: vaultPDA,
        instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction()
  );

  return tx;
}

// ─────────────────────────────────────────────────────────────
// BALANCE LOGGING
// ─────────────────────────────────────────────────────────────
export async function logBalances(
  connection: Connection,
  accounts: { label: string; pubkey: PublicKey }[]
): Promise<Map<string, number>> {
  const balances = new Map<string, number>();
  for (const { label, pubkey } of accounts) {
    const bal = await connection.getBalance(pubkey);
    balances.set(label, bal);
    console.log(`    ${label}: ${bal / LAMPORTS_PER_SOL} SOL`);
  }
  return balances;
}

// ─────────────────────────────────────────────────────────────
// BET CALCULATION
//
// Mirrors on-chain: prize_pool * bet_bps / 10_000
// ─────────────────────────────────────────────────────────────
export function calcBet(prizePool: number, betBps: number): number {
  return Math.floor((prizePool * betBps) / 10_000);
}

// ─────────────────────────────────────────────────────────────
// WAIT FOR ROLL
//
// Polls GameState.roll_ready until true or timeout.
// The VRF oracle calls callback_randomness() which sets
// roll_ready = true on-chain (~5-10s on devnet).
// ─────────────────────────────────────────────────────────────
export async function waitForRoll(
  program: Program<PayToGuess>,
  gamePDA: PublicKey,
  timeoutMs = 30_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 2000));
    const state = await program.account.gameState.fetch(gamePDA);
    if (state.rollReady) return;
  }
  throw new Error("VRF roll timeout — oracle did not respond in time");
}

export async function getErValidator(baseUrl: string): Promise<PublicKey> {
  const response = await fetch(baseUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getIdentity",
      params: [],
    }),
  });

  if (!response.ok) {
    throw new Error(`HTTP error: ${response.status}`);
  }

  const data = (await response.json()) as JsonRpcResponse;

  if (data.error) {
    throw new Error(`getIdentity failed: ${JSON.stringify(data.error)}`);
  }

  const identity = data.result?.identity;

  if (!identity) {
    throw new Error(
      `getIdentity returned no identity: ${JSON.stringify(data.result)}`
    );
  }

  return new PublicKey(identity);
}
type JsonRpcResponse = {
  jsonrpc: string;
  id: number;
  result?: {
    identity: string;
  };
  error?: any;
};

// ─────────────────────────────────────────────────────────────
// ERROR ASSERTION
//
// Expects the promise to throw an AnchorError with a specific
// error code. Works for both .rpc() and sendAndConfirmTransaction.
// ─────────────────────────────────────────────────────────────
export async function expectAnchorError(
  promise: Promise<any>,
  errorCode: string
): Promise<void> {
  try {
    await promise;
    throw new Error(`Expected error "${errorCode}" but transaction succeeded`);
  } catch (err: any) {
    if (err instanceof anchor.AnchorError) {
      expect(err.error.errorCode.code).to.equal(
        errorCode,
        `Expected "${errorCode}" but got "${err.error.errorCode.code}"`
      );
      console.log(`    rejected: ${errorCode}`);
    } else {
      // sendAndConfirmTransaction wraps errors — parse from logs/message
      const msg = err?.logs?.join(" ") ?? err?.message ?? String(err);
      expect(msg).to.include(
        errorCode,
        `Expected "${errorCode}" in error but got: ${msg.slice(0, 200)}`
      );
      console.log(`    rejected: ${errorCode}`);
    }
  }
}
