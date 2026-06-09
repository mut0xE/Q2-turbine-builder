use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct PlayerState {
    pub player: Pubkey,
    pub current_guess: u8,
    pub previous_guess: u8,
    pub current_paid: u64,
    pub total_rounds: u64,
    pub total_wins: u64,
    pub bump: u8,
}
