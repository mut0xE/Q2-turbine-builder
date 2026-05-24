import * as anchor from "@coral-xyz/anchor";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

/**
 * Creates a new SPL token mint.
 */
export async function createTokenMint(
  connection: Connection,
  payer: Keypair,
  decimals: number = 6
): Promise<PublicKey> {
  return createMint(connection, payer, payer.publicKey, null, decimals);
}

/**
 * Creates an associated token account for a given mint and owner.
 */
export async function createAta(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
): Promise<PublicKey> {
  return createAssociatedTokenAccount(connection, payer, mint, owner);
}

/**
 * Mints tokens to a destination token account.
 */
export async function mintTokens(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  destination: PublicKey,
  amount: number | bigint
): Promise<string> {
  return mintTo(connection, payer, mint, destination, payer, amount);
}

/**
 * Derives the associated token address (sync, no RPC call).
 */
export function getAta(mint: PublicKey, owner: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, owner, true);
}

/**
 * Full setup: create mint -> create ATA -> mint tokens.
 * Returns { mint, ata }.
 */
export async function setupMintAndFund(
  connection: Connection,
  payer: Keypair,
  amount: number | bigint,
  decimals: number = 6
): Promise<{ mint: PublicKey; ata: PublicKey }> {
  const mint = await createTokenMint(connection, payer, decimals);
  const ata = await createAta(connection, payer, mint, payer.publicKey);
  await mintTokens(connection, payer, mint, ata, amount);
  return { mint, ata };
}
