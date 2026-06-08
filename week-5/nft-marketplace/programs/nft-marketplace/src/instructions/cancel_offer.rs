use crate::constants::OFFER_VAULT;
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

    /// CHECK: Vault that holds the escrowed SOL
    #[account(
            mut,
            seeds = [OFFER_VAULT, offer.key().as_ref()],
            bump = offer.vault_bump,
        )]
    pub offer_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CancelOffer>) -> Result<()> {
    let offer_key = ctx.accounts.offer.key();
    let vault_bump = ctx.accounts.offer.vault_bump;
    let refund_amount = ctx.accounts.offer.amount;

    // vault PDA signs to release SOL back to taker
    let vault_seeds: &[&[&[u8]]] = &[&[OFFER_VAULT, offer_key.as_ref(), &[vault_bump]]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.offer_vault.to_account_info(),
                to: ctx.accounts.taker.to_account_info(),
            },
            vault_seeds,
        ),
        refund_amount,
    )?;

    msg!("Offer cancelled: {} lamports refunded", refund_amount);

    Ok(())
}
