use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub taker: Pubkey, //seller
    pub asset: Pubkey, //NFT pubkey
    pub amount: u64,   //lamports (SOL) or token amount
    pub payment_mint: Option<Pubkey>,
    pub bump: u8,
    pub vault_bump: u8,
}
