import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { randomBytes } from "crypto";
import { BN, Program } from "@coral-xyz/anchor";
import { DungeonVault } from "../target/types/dungeon_vault";

export const DEFAULT_QUEUE = new PublicKey(
  "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc"
);

export const ER_URL = "https://devnet-as.magicblock.app/";

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

type JsonRpcResponse = {
  jsonrpc: string;
  id: number;
  result?: {
    identity: string;
  };
  error?: any;
};

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

export async function submitChoiceOnER(
  program: Program<DungeonVault>,
  erConnection: anchor.web3.Connection,
  dungeonId: BN,
  dungeonPDA: PublicKey,
  playerStatePDA: PublicKey,
  player: Keypair,
  choice: number
): Promise<string> {
  const ix = await program.methods
    .submitChoice(dungeonId, choice)
    .accounts({
      player: player.publicKey,
      //@ts-ignore
      dungeon: dungeonPDA,
      playerState: playerStatePDA,
    })
    .instruction();

  const tx = new Transaction().add(ix);
  tx.feePayer = player.publicKey;
  tx.recentBlockhash = (await erConnection.getLatestBlockhash()).blockhash;

  return sendAndConfirmTransaction(erConnection, tx, [player], {
    skipPreflight: true,
    commitment: "confirmed",
  });
}

export async function getDungeonFromER(
  erConnection: anchor.web3.Connection,
  program: Program<DungeonVault>,
  dungeonPDA: PublicKey
) {
  const info = await erConnection.getAccountInfo(dungeonPDA);
  if (!info) throw new Error("Dungeon account not found on ER");
  return program.coder.accounts.decode("dungeon", info.data);
}

export async function getPlayerStateFromER(
  erConnection: anchor.web3.Connection,
  program: Program<DungeonVault>,
  playerStatePDA: PublicKey
) {
  const info = await erConnection.getAccountInfo(playerStatePDA);
  if (!info) throw new Error("PlayerState account not found on ER");
  return program.coder.accounts.decode("playerState", info.data);
}

export async function requestVrfAndWait(
  program: Program<DungeonVault>,
  erConnection: anchor.web3.Connection,
  dungeonId: BN,
  dungeonPDA: PublicKey,
  authority: Keypair,
  waitMs = 10_000
): Promise<string> {
  const callerSeed = randomBytes(1)[0];

  const requestIx = await program.methods
    .requestRandomness(dungeonId, callerSeed)
    .accounts({
      payer: authority.publicKey,
      //@ts-ignore
      dungeon: dungeonPDA,
      oracleQueue: DEFAULT_QUEUE,
    })
    .instruction();

  const tx = new Transaction().add(requestIx);
  tx.feePayer = authority.publicKey;
  tx.recentBlockhash = (await erConnection.getLatestBlockhash()).blockhash;

  const sig = await sendAndConfirmTransaction(erConnection, tx, [authority], {
    skipPreflight: true,
    commitment: "confirmed",
  });

  console.log(`  Waiting ${waitMs / 1000}s for VRF callback...`);
  await new Promise((r) => setTimeout(r, waitMs));

  return sig;
}
