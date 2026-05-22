pub const SEED_AMM_CONFIG: &[u8] = b"amm_config";
pub const SEED_POOL: &[u8] = b"pool";
pub const SEED_LP_MINT: &[u8] = b"lp_mint";
pub const SEED_VAULT_X: &[u8] = b"vault_x";
pub const SEED_VAULT_Y: &[u8] = b"vault_y";

/// actual_percentage = fee_rate / FEE_DENOMINATOR * 100
pub const FEE_DENOMINATOR: u64 = 10_000; // basis points denominator
pub const MAX_FEE: u16 = 10_000; // 100% — blocked at this value
pub const DEFAULT_FEE: u16 = 30; // 0.3%

pub const LP_DECIMALS: u8 = 6; // LP token decimal places

// anchor discriminator
pub const DISCRIMINATOR: usize = 8;
