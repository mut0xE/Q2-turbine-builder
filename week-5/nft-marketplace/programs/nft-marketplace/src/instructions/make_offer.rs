use crate::{
    constants::{LISTING_SEED, MARKET_SEED, OFFER_SEED, OFFER_VAULT},
    error::MarketPlaceError,
    state::{Listing, MarketPlace, Offer},
};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct MakeOffer<'info> {
    // Buyer making the offer
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: just the asset pubkey for seeds
    pub asset: UncheckedAccount<'info>,

    #[account(
        seeds = [MARKET_SEED, market_place.name.as_str().as_bytes()],
        bump = market_place.bump
    )]
    pub market_place: Account<'info, MarketPlace>,

    #[account(
        seeds = [
            LISTING_SEED,
            asset.key().as_ref()
        ],
        bump = listing.bump,
        constraint = listing.asset == asset.key()
            @ MarketPlaceError::ListingNotFound,
    )]
    pub listing: Account<'info, Listing>,

    // Offer PDA — escrows the buyer's SOL
    // seeds: [b"offer", asset, taker]
    // One offer per buyer per asset
    #[account(
        init,
        payer = taker,
        space = Offer::DISCRIMINATOR.len() + Offer::INIT_SPACE,
        seeds = [
            OFFER_SEED,
            asset.key().as_ref(),
            taker.key().as_ref()
        ],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        seeds = [
            OFFER_VAULT,
            offer.key().as_ref()
        ],
        bump
    )]
    pub offer_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<MakeOffer>, amount: u64) -> Result<()> {
    require!(amount > 0, MarketPlaceError::InvalidOfferAmount);

    ctx.accounts.offer.set_inner(Offer {
        taker: ctx.accounts.taker.key(),
        asset: ctx.accounts.asset.key(),
        amount,
        payment_mint: None,
        bump: ctx.bumps.offer,
        vault_bump: ctx.bumps.offer_vault,
    });

    // Transfer SOL from taker to offer Vault PDA (escrow)
    // SOL sits here until accept or cancel
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.taker.to_account_info(),
                to: ctx.accounts.offer_vault.to_account_info(),
            },
        ),
        amount,
    )?;

    msg!(
        "Offer made: {} lamports for asset {}",
        amount,
        ctx.accounts.asset.key()
    );

    Ok(())
}
