use crate::{constants::OFFER_SEED, error::MarketPlaceError, state::Offer};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    // Only the buyer who made the offer can cancel
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: just the asset pubkey for seeds
    pub asset: UncheckedAccount<'info>,

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
        close = taker, // rent back to taker
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CancelOffer>) -> Result<()> {
    let asset_key = ctx.accounts.asset.key();
    let taker_key = ctx.accounts.taker.key();
    let offer_bump = ctx.accounts.offer.bump;
    let refund_amount = ctx.accounts.offer.amount;

    // Transfer SOL from offer PDA back to taker
    let offer_seeds: &[&[&[u8]]] = &[&[
        OFFER_SEED,
        asset_key.as_ref(),
        taker_key.as_ref(),
        &[offer_bump],
    ]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer.to_account_info(),
                to: ctx.accounts.taker.to_account_info(),
            },
            offer_seeds,
        ),
        refund_amount,
    )?;

    // offer account closed automatically (close = taker)
    msg!("Offer cancelled for asset: {}", asset_key);
    msg!(
        "Refunded {} lamports to taker: {}",
        refund_amount,
        taker_key
    );

    Ok(())
}
