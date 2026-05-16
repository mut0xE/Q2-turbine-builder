use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct PlayerState {
    pub player: Pubkey,
    pub dungeon: Pubkey,
    pub alive: bool,
    pub current_choice: u8,
    pub bump: u8,
}
