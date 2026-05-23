use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AmmConfig {
    pub fee_rate: u16,             // basis points e.g. 30 = 0.3%
    pub authority: Option<Pubkey>, // who can update this config
    pub index: u16,                // unique index, used as PDA seed
    pub bump: u8,                  // PDA bump saved so we don't recompute
}

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub config: Pubkey,
    pub mint_x: Pubkey,  // token X mint address
    pub mint_y: Pubkey,  // token Y mint address
    pub lp_mint: Pubkey, // LP token mint, owned by pool PDA
    pub pool_bump: u8,
    pub vault_x_bump: u8,
    pub vault_y_bump: u8,
    pub locked: bool,
    pub lp_bump: u8,
}
