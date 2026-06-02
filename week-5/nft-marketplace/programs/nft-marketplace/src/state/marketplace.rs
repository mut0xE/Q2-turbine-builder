use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct MarketPlace {
    pub admin: Pubkey,
    pub fee: u16,
    pub treasury: Pubkey,
    pub rewards_mint: Pubkey,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
    #[max_len(32)]
    pub name: String,
    pub bump: u8,
}
