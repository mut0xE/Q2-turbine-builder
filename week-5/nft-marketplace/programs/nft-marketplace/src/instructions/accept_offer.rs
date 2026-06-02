use crate::{
    constants::{LISTING_SEED, MARKET_SEED, OFFER_SEED, TREASURY_SEED},
    error::MarketPlaceError,
    state::{Listing, MarketPlace, Offer},
};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    // Seller accepts the offer
    #[account(mut)]
    pub maker: Signer<'info>,

    // Buyer who made the offer receives NFT
    /// CHECK: verified via offer.taker
    #[account(mut)]
    pub taker: UncheckedAccount<'info>,

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
        constraint = listing.asset == asset.key()
            @ MarketPlaceError::ListingNotFound,
        close = maker,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        seeds = [
            OFFER_SEED,
            asset.key().as_ref(),
            taker.key().as_ref()
        ],
        bump = offer.bump,
        constraint = offer.taker == taker.key()
            @ MarketPlaceError::Unauthorized,
        constraint = offer.asset == asset.key()
            @ MarketPlaceError::OfferNotFound,
        close = taker,
    )]
    pub offer: Account<'info, Offer>,

    // Treasury receives fee from offer amount
    #[account(
        mut,
        seeds = [TREASURY_SEED, market_place.admin.as_ref()],
        bump = market_place.treasury_bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<AcceptOffer>) -> Result<()> {
    let market_place_key = ctx.accounts.market_place.key();
    let asset_key = ctx.accounts.asset.key();
    let listing_bump = ctx.accounts.listing.bump;
    let offer_amount = ctx.accounts.offer.amount;

    // STEP 1 — Calculate fee from offer amount
    let fee_amount = offer_amount
        .checked_mul(ctx.accounts.market_place.fee as u64)
        .ok_or(MarketPlaceError::InvalidOfferAmount)?
        .checked_div(10_000)
        .ok_or(MarketPlaceError::InvalidOfferAmount)?;

    let maker_amount = offer_amount
        .checked_sub(fee_amount)
        .ok_or(MarketPlaceError::InvalidOfferAmount)?;

    // STEP 2 — Transfer SOL from offer PDA to maker
    // offer PDA signs with its seeds
    let asset_key_ref = ctx.accounts.asset.key();
    let taker_key_ref = ctx.accounts.taker.key();
    let offer_bump = ctx.accounts.offer.bump;

    let offer_seeds: &[&[&[u8]]] = &[&[
        OFFER_SEED,
        asset_key_ref.as_ref(),
        taker_key_ref.as_ref(),
        &[offer_bump],
    ]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer.to_account_info(),
                to: ctx.accounts.maker.to_account_info(),
            },
            offer_seeds,
        ),
        maker_amount,
    )?;

    // STEP 3 — Transfer fee from offer PDA to treasury
    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
            offer_seeds,
        ),
        fee_amount,
    )?;

    // STEP 4 — Transfer NFT: listing PDA to taker
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        market_place_key.as_ref(),
        asset_key.as_ref(),
        &[listing_bump],
    ]];

    let listing_info = &ctx.accounts.listing.to_account_info();
    let taker_info = &ctx.accounts.taker.to_account_info();

    let mut builder = TransferV1CpiBuilder::new(&ctx.accounts.mpl_core_program);

    builder
        .asset(&ctx.accounts.asset)
        .payer(&ctx.accounts.maker)
        .authority(Some(listing_info))
        .new_owner(taker_info)
        .system_program(Some(&ctx.accounts.system_program));

    if let Some(collection) = &ctx.accounts.collection {
        builder.collection(Some(collection));
    }

    builder.invoke_signed(listing_seeds)?;

    msg!("Offer accepted for asset: {}", ctx.accounts.asset.key());
    msg!("Taker: {}", ctx.accounts.taker.key());
    msg!("Offer amount: {}", offer_amount);
    msg!("Maker received: {}", maker_amount);
    msg!("Fee: {}", fee_amount);

    Ok(())
}
