use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub maker: Pubkey, //seller
    pub asset: Pubkey, //NFT pubkey
    pub price: u64,    //lamports (SOL) or token amount
    pub payment_mint: Option<Pubkey>,
    pub bump: u8,
}
