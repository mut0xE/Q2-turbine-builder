use anchor_lang::prelude::*;

#[error_code]
pub enum MarketPlaceError {
    #[msg("Invalid marketplace fee")]
    InvalidFee,

    #[msg("Invalid marketplace Name")]
    InvalidName,

    #[msg("Only the maker can perform this action")]
    Unauthorized,

    #[msg("Listing already exists")]
    ListingAlreadyExists,

    #[msg("Listing not found")]
    ListingNotFound,

    #[msg("Invalid payment mint")]
    WrongPaymentMint,

    #[msg("Offer not found")]
    OfferNotFound,

    #[msg("Offer amount must be greater than zero")]
    InvalidOfferAmount,

    #[msg("Insufficient funds")]
    InsufficientFunds,

    #[msg("Price must be greater than zero")]
    InvalidPrice,
}
