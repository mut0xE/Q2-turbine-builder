use anchor_lang::prelude::*;

#[error_code]
pub enum StakingError {
    #[msg("Asset does not belong to the expected collection")]
    WrongCollection,

    #[msg("You are not the owner of this asset")]
    Unauthorized,

    #[msg("Asset is not currently staked")]
    NotStaked,

    #[msg("Asset is already staked")]
    AlreadyStaked,

    #[msg("Freeze period has not passed yet")]
    FreezePeriodNotPassed,

    #[msg("Overflow when calculating rewards")]
    Overflow,
}
