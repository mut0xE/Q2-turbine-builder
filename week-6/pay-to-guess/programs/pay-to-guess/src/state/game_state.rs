use anchor_lang::prelude::*;
#[account]
#[derive(InitSpace)]
pub struct GameState {
    pub authority: Pubkey,
    pub secret: u8,
    pub prize_pool: u64,
    pub bet_amount: u64,
    pub total_rounds: u64,
    pub bump: u8,
    pub vault_bump: u8,
}
