use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Fee rate cannot exceed 10000 basis points")]
    InvalidFee,

    #[msg("Deposit amounts must match the current pool ratio")]
    InvalidRatio,

    #[msg("Slippage tolerance exceeded — output below minimum")]
    SlippageExceeded,

    #[msg("Cannot swap zero tokens")]
    ZeroAmount,

    #[msg("Pool already exists for this token pair")]
    PoolAlreadyExists,

    #[msg("LP amount cannot be zero")]
    ZeroLpAmount,

    #[msg("Overflow in math calculation")]
    MathOverflow,

    #[msg("Pool is locked — deposits and swaps are paused")]
    PoolLocked,

    #[msg("Invalid mint")]
    InvalidMint,

    #[msg("Invalid vault owner")]
    InvalidVaultOwner,
}
