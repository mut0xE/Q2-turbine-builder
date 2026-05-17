// programs/dungeon-vault/src/errors.rs

use anchor_lang::prelude::*;

#[error_code]
pub enum DungeonError {
    #[msg("Invalid entry fee")]
    InvalidEntryFee,

    #[msg("InsufficientFunds")]
    InsufficientFunds,

    #[msg("AlreadyClaimed")]
    AlreadyClaimed,

    #[msg("Dungeon is already full")]
    DungeonFull,

    #[msg("Game has already started")]
    GameAlreadyStarted,

    #[msg("Game is not active")]
    GameNotActive,

    #[msg("Invalid choice")]
    InvalidChoice,

    #[msg("Player already submitted choice")]
    AlreadySubmitted,

    #[msg("Game is not finished")]
    GameNotFinished,

    #[msg("Player is eliminated")]
    PlayerEliminated,

    #[msg("Reward already claimed")]
    RewardAlreadyClaimed,

    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Not enough players to start")]
    NotEnoughPlayers,

    #[msg("Overflow")]
    Overflow,

    #[msg("Underflow")]
    Underflow,
}
