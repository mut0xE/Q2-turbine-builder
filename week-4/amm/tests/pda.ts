import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

// Must match the program ID in lib.rs / Anchor.toml
const PROGRAM_ID = new PublicKey(
  "9skP2HrosgroRxykvVwF1K4w4FJPTxeuJSpHrgcvRrDK"
);

// Seeds must match programs/amm/src/constants.rs
const SEED_AMM_CONFIG = Buffer.from("amm_config");
const SEED_POOL = Buffer.from("pool");
const SEED_LP_MINT = Buffer.from("lp_mint");
const SEED_VAULT_X = Buffer.from("vault_x");
const SEED_VAULT_Y = Buffer.from("vault_y");

export function findConfigPda(index: BN): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_AMM_CONFIG, index.toArrayLike(Buffer, "le", 8)],
    PROGRAM_ID
  );
}

export function findPoolPda(
  config: PublicKey,
  mintX: PublicKey,
  mintY: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_POOL, config.toBuffer(), mintX.toBuffer(), mintY.toBuffer()],
    PROGRAM_ID
  );
}

export function findLpMintPda(pool: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_LP_MINT, pool.toBuffer()],
    PROGRAM_ID
  );
}

export function findVaultXPda(
  pool: PublicKey,
  mintX: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_VAULT_X, pool.toBuffer(), mintX.toBuffer()],
    PROGRAM_ID
  );
}

export function findVaultYPda(
  pool: PublicKey,
  mintY: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_VAULT_Y, pool.toBuffer(), mintY.toBuffer()],
    PROGRAM_ID
  );
}

/**
 * Derives all PDAs needed for the AMM pool in one call.
 */
export function deriveAllPdas(
  index: BN,
  mintX: PublicKey,
  mintY: PublicKey
) {
  const [config] = findConfigPda(index);
  const [pool] = findPoolPda(config, mintX, mintY);
  const [lpMint] = findLpMintPda(pool);
  const [vaultX] = findVaultXPda(pool, mintX);
  const [vaultY] = findVaultYPda(pool, mintY);

  return { config, pool, lpMint, vaultX, vaultY };
}
