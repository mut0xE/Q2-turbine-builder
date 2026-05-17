import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

const DUNGEON_SEED = Buffer.from("dungeon");
const PLAYER_SEED = Buffer.from("player");
const VAULT_SEED = Buffer.from("vault");

export function getDungeonPDA(
  dungeonId: BN,
  authority: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      DUNGEON_SEED,
      dungeonId.toArrayLike(Buffer, "le", 8),
      authority.toBuffer(),
    ],
    programId
  );
}

export function getVaultPDA(
  dungeonPDA: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, dungeonPDA.toBuffer()],
    programId
  );
}

export function getPlayerStatePDA(
  dungeonPDA: PublicKey,
  player: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [PLAYER_SEED, dungeonPDA.toBuffer(), player.toBuffer()],
    programId
  );
}
