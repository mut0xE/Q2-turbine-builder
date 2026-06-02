use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub rewards_bps: u16,   // reward rate in basis points per day (100 = 1%)
    pub freeze_period: u16, // minimum days NFT must stay staked before unstake
    pub rewards_bump: u8,   // bump for the reward token mint PDA
    pub bump: u8,           // bump for this config PDA
}
