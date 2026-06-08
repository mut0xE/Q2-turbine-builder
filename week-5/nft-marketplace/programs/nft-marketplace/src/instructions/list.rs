use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::{
    constants::{LISTING_SEED, MARKET_SEED},
    error::MarketPlaceError,
    state::{Listing, MarketPlace},
};
#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        init,
        space = Listing::DISCRIMINATOR.len() + Listing::INIT_SPACE,
        payer = maker,
        seeds = [
            LISTING_SEED,
            market_place.key().as_ref(),
            asset.key().as_ref()
        ],
        bump
    )]
    pub listing: Account<'info, Listing>,

    /// CHECK: validated by MPL Core CPI
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: validated by MPL Core CPI
    #[account(mut)]
    pub collection: Option<UncheckedAccount<'info>>,

    #[account(
        mut,
        seeds = [
            MARKET_SEED,
            market_place.name.as_str().as_bytes()
        ],
        bump = market_place.bump
    )]
    pub market_place: Account<'info, MarketPlace>,

    // payment_mint is None for SOL listings
    // payment_mint is Some(mint) for SPL token listings
    pub payment_mint: Option<InterfaceAccount<'info, Mint>>,

    pub system_program: Program<'info, System>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program ID
    pub mpl_core_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<List>, price: u64) -> Result<()> {
    require!(price > 0, MarketPlaceError::InvalidPrice);

    ctx.accounts.listing.set_inner(Listing {
        maker: ctx.accounts.maker.key(),
        asset: ctx.accounts.asset.key(),
        price,
        payment_mint: ctx.accounts.payment_mint.as_ref().map(|m| m.key()),
        bump: ctx.bumps.listing,
    });
    // Transfer NFT from maker to listing PDA
    // TransferV1CpiBuilder moves ownership inside MPL Core
    // After this: asset.owner = listing.key()
    let mut builder = TransferV1CpiBuilder::new(&ctx.accounts.mpl_core_program);
    let listing = &ctx.accounts.listing.to_account_info();
    builder
        .asset(&ctx.accounts.asset)
        .payer(&ctx.accounts.maker)
        .authority(Some(&ctx.accounts.maker)) // current owner signs
        .new_owner(listing)
        .system_program(Some(&ctx.accounts.system_program));
    if let Some(collection) = &ctx.accounts.collection {
        builder.collection(Some(collection));
    }
    builder.invoke()?;

    msg!("Listed asset: {}", ctx.accounts.asset.key());
    msg!("New owner (escrow): {}", ctx.accounts.listing.key());
    msg!("Price: {}", price);
    msg!(
        "Payment mint: {:?}",
        ctx.accounts.payment_mint.as_ref().map(|m| m.key())
    );
    Ok(())
}
