use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("SOL amount must be greater than 0")]
    InvalidSolAmount,
    #[msg("USDC amount must be greater than 0")]
    InvalidUsdcAmount,
    #[msg("Escrow is not open")]
    NotOpen,
    #[msg("Only maker can cancel")]
    Unauthorized,
}
