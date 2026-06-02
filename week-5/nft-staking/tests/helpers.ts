import { Connection, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

// --- PDA Derivation Helpers ---

// seeds: [b"config"]
export function getConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from("config")], programId);
}

// seeds: [b"update_authority", collection]
export function getUpdateAuthorityPda(
  collection: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collection.toBuffer()],
    programId
  );
}

// seeds: [b"rewards", config]
export function getRewardMintPda(
  config: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    programId
  );
}

// seeds: [b"stake", asset, owner]
export function getStakeInfoPda(
  asset: PublicKey,
  owner: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("stake"), asset.toBuffer(), owner.toBuffer()],
    programId
  );
}

// --- Utility Helpers ---

export async function airdrop(
  connection: Connection,
  pubkey: PublicKey,
  sol: number = 10
) {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(sig);
}

export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// --- Logging Helpers ---

const SEPARATOR = "---------------------------------------------";

export function logTxSignature(label: string, signature: string) {
  console.log(`  [tx] ${label}`);
  console.log(`        sig: ${signature}`);
}

export function logAccount(label: string, address: PublicKey | string) {
  const addr = typeof address === "string" ? address : address.toBase58();
  console.log(`  [account] ${label}: ${addr}`);
}

export function logStakeDetails(details: {
  owner: PublicKey;
  asset: PublicKey;
  collection: PublicKey;
  stakedAt: number;
  lastClaimed: number;
}) {
  console.log(`  [stake-info]`);
  console.log(`        owner:        ${details.owner.toBase58()}`);
  console.log(`        asset:        ${details.asset.toBase58()}`);
  console.log(`        collection:   ${details.collection.toBase58()}`);
  console.log(`        staked_at:    ${details.stakedAt}`);
  console.log(`        last_claimed: ${details.lastClaimed}`);
}

export function logConfigDetails(details: {
  rewardsBps: number;
  freezePeriod: number;
  rewardMint: PublicKey;
  configPda: PublicKey;
}) {
  console.log(`  [config]`);
  console.log(`        config PDA:   ${details.configPda.toBase58()}`);
  console.log(`        reward mint:  ${details.rewardMint.toBase58()}`);
  console.log(
    `        rewards bps:  ${details.rewardsBps} (${
      details.rewardsBps / 100
    }%/day)`
  );
  console.log(`        freeze period: ${details.freezePeriod} days`);
}

export function logSection(title: string) {
  console.log(`\n  ${SEPARATOR}`);
  console.log(`  ${title}`);
  console.log(`  ${SEPARATOR}`);
}

export function logError(label: string, err: unknown) {
  const msg = err instanceof Error ? err.message.split("\n")[0] : String(err);
  console.log(`  [expected-error] ${label}: ${msg}`);
}
