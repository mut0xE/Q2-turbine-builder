import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Transaction,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { randomBytes } from "crypto";

export async function airdrop(
  provider: anchor.Provider,
  from: Keypair,
  to: PublicKey
) {
  const fundTx = new Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from.publicKey,
      toPubkey: to,
      lamports: 0.02 * LAMPORTS_PER_SOL,
    })
  );
  const fundSig = await provider.sendAndConfirm(fundTx, [from]);

  console.log(`funded: ${fundSig}`);
}

export function getDungeonId(): anchor.BN {
  return new anchor.BN(randomBytes(8));
}
