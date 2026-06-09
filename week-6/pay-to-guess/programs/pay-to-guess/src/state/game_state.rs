use anchor_lang::prelude::*;
#[account]
#[derive(InitSpace)]
pub struct GameState {
    pub authority: Pubkey,
    pub prize_pool: u64,
    pub bet_amount: u64,
    pub bet_bps: u16,
    pub total_rounds: u64,
    pub bump: u8,
    pub vault_bump: u8,
    pub current_roll: u8, // VRF result stored here
    pub roll_ready: bool, // true = roll exists, player can play
}
