import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import fs from "fs";
// Load player from file
export function loadPlayer(filePath: string): Keypair {
  const data = JSON.parse(fs.readFileSync(filePath, "utf8"));
  return Keypair.fromSecretKey(Uint8Array.from(data));
}

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

  return await sendAndConfirmTransaction(connection, tx, [payer]);
}

export function getMarketplacePda(
  name: string,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("market_place"), Buffer.from(name)],
    programId
  );
}

export function getTreasuryPda(
  admin: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), admin.toBuffer()],
    programId
  );
}

export function getRewardsMintPda(
  marketplace: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), marketplace.toBuffer()],
    programId
  );
}

export function getListingPda(
  asset: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("listing"), asset.toBuffer()],
    programId
  );
}

export function getOfferPda(
  asset: PublicKey,
  taker: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("offer"), asset.toBuffer(), taker.toBuffer()],
    programId
  );
}

export function getOfferVaultPda(
  offer: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("offer_vault"), offer.toBuffer()],
    programId
  );
}
