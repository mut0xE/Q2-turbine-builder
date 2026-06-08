use crate::{
    constants::{LISTING_SEED, MARKET_SEED},
    error::MarketPlaceError,
    state::{Listing, MarketPlace},
};
use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct Delist<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: validated by MPL Core
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: validated by MPL Core
    #[account(mut)]
    pub collection: Option<UncheckedAccount<'info>>,

    #[account(
        seeds = [MARKET_SEED, market_place.name.as_str().as_bytes()],
        bump = market_place.bump
    )]
    pub market_place: Account<'info, MarketPlace>,

    // listing PDA
    #[account(
        mut,
        seeds = [
            LISTING_SEED,
            market_place.key().as_ref(),
            asset.key().as_ref()
        ],
        bump = listing.bump,
        constraint = listing.maker == maker.key()
            @ MarketPlaceError::Unauthorized,
        constraint = listing.asset == asset.key() @ MarketPlaceError::ListingNotFound,
        close = maker,
    )]
    pub listing: Account<'info, Listing>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Delist>) -> Result<()> {
    let market_place_key = ctx.accounts.market_place.key();
    let asset_key = ctx.accounts.asset.key();
    let listing_bump = ctx.accounts.listing.bump;

    // listing PDA signer seeds
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        market_place_key.as_ref(),
        asset_key.as_ref(),
        &[listing_bump],
    ]];

    let mut builder = TransferV1CpiBuilder::new(&ctx.accounts.mpl_core_program);

    let listing = &ctx.accounts.listing.to_account_info();
    builder
        .asset(&ctx.accounts.asset)
        .payer(&ctx.accounts.maker)
        .authority(Some(listing)) // current owner signs
        .new_owner(&ctx.accounts.maker)
        .system_program(Some(&ctx.accounts.system_program));
    if let Some(collection) = &ctx.accounts.collection {
        builder.collection(Some(collection));
    }
    builder.invoke_signed(listing_seeds)?;

    // listing closed automatically (close = maker)
    msg!("Delisted: {}", ctx.accounts.asset.key());
    Ok(())
}
