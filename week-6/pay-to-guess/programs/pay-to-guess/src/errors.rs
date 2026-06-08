use anchor_lang::prelude::*;
#[error_code]
pub enum GameError {
    #[msg("Prize pool must be > 0")]
    InvalidPrizePool,
    #[msg("Guess must be 1–6")]
    InvalidGuess,
    #[msg("Introspection sysvar read failed")]
    IntrospectionFailed,
    #[msg("No payment instruction before play()")]
    MissingPaymentInstruction,
    #[msg("Preceding instruction is not from System Program")]
    PaymentNotSystemProgram,
    #[msg("Preceding instruction is not a Transfer")]
    NotATransfer,
    #[msg("Payment amount is below bet_amount")]
    InsufficientPayment,
    #[msg("Payment destination is not the game vault")]
    WrongPaymentDestination,
    #[msg("Only the game authority can do this")]
    Unauthorized,
    #[msg("Math overflow")]
    MathOverflow,
}
