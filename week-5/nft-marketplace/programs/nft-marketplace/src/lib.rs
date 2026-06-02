use anchor_lang::prelude::*;

declare_id!("3McokwkwSf5UFtgxiHRXbG1o9A7AmDdMajLn4YtbvR4U");
mod constants;
mod error;
mod instructions;
mod state;

use instructions::*;
#[program]
pub mod nft_marketplace {
    use crate::instructions::{
        accept_offer, buy, buy_with_token, cancel_offer, delist, initialize, list, make_offer,
    };

    use super::*;
    pub fn initialize(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        initialize::handler(ctx, name, fee)
    }

    pub fn list(ctx: Context<List>, price: u64) -> Result<()> {
        list::handler(ctx, price)
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        delist::handler(ctx)
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        buy::handler(ctx)
    }

    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        buy_with_token::handler(ctx)
    }

    pub fn make_offer(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
        make_offer::handler(ctx, amount)
    }

    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        accept_offer::handler(ctx)
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        cancel_offer::handler(ctx)
    }
}
