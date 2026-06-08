use crate::{
    constants::{LISTING_SEED, MARKET_SEED, REWARDS_SEED, TREASURY_SEED},
    error::MarketPlaceError,
    state::{Listing, MarketPlace},
};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};

use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct Buy<'info> {
    // Buyer pays SOL
    #[account(mut)]
    pub taker: Signer<'info>,

    // Seller receives SOL
    /// CHECK: just receives SOL
    #[account(mut)]
    pub maker: SystemAccount<'info>,

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
        seeds = [REWARDS_SEED, market_place.key().as_ref()],
        bump = market_place.rewards_bump,
        mint::decimals = 6,
        mint::authority = market_place
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    // Taker's ATA for reward tokens
    #[account(
            init_if_needed,
            payer = taker,
            associated_token::mint = rewards_mint,
            associated_token::authority = taker,
            associated_token::token_program = token_program,
        )]
    pub taker_rewards_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            LISTING_SEED,
            market_place.key().as_ref(),
            asset.key().as_ref()
        ],
        bump = listing.bump,
        // verify listing belongs to this asset
        constraint = listing.asset == asset.key()
            @ MarketPlaceError::ListingNotFound,
        // verify maker matches listing
        constraint = listing.maker == maker.key()
            @ MarketPlaceError::Unauthorized,
        // SOL listing only — no payment_mint
        constraint = listing.payment_mint.is_none()
            @ MarketPlaceError::WrongPaymentMint,
        close = maker, // listing rent to maker after buy
    )]
    pub listing: Account<'info, Listing>,

    // Treasury receives fee
    #[account(
        mut,
        seeds = [TREASURY_SEED, market_place.admin.as_ref()],
        bump = market_place.treasury_bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program
    pub mpl_core_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Buy>) -> Result<()> {
    let market_place_key = ctx.accounts.market_place.key();
    let asset_key = ctx.accounts.asset.key();
    let listing_bump = ctx.accounts.listing.bump;
    let price = ctx.accounts.listing.price;

    // STEP 1 — Calculate fee
    //
    // fee = price * marketplace.fee / 10_000
    // maker receives = price - fee
    // treasury receives = fee
    //
    // Example: price=1_000_000, fee=250 (2.5%)
    //   fee_amount = 1_000_000 * 250 / 10_000 = 25_000
    //   maker gets = 975_000
    // ----------------------------------------------------------
    let fee_amount = price
        .checked_mul(ctx.accounts.market_place.fee as u64)
        .ok_or(MarketPlaceError::InvalidPrice)?
        .checked_div(10_000)
        .ok_or(MarketPlaceError::InvalidPrice)?;

    let maker_amount = price
        .checked_sub(fee_amount)
        .ok_or(MarketPlaceError::InvalidPrice)?;

    // STEP 2 — Transfer SOL: buyer to maker
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.taker.to_account_info(),
                to: ctx.accounts.maker.to_account_info(),
            },
        ),
        maker_amount,
    )?;

    // STEP 3 — Transfer fee: buyer to treasury
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.taker.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        ),
        fee_amount,
    )?;

    // STEP 4 — Transfer NFT: listing PDA to buyer
    //
    // listing PDA is current owner
    // must sign with listing seeds
    let listing_seeds: &[&[&[u8]]] = &[&[
        LISTING_SEED,
        market_place_key.as_ref(),
        asset_key.as_ref(),
        &[listing_bump],
    ]];

    let mut builder = TransferV1CpiBuilder::new(&ctx.accounts.mpl_core_program);
    let listing = &ctx.accounts.listing.to_account_info();
    let taker = &ctx.accounts.taker.to_account_info();
    builder
        .asset(&ctx.accounts.asset)
        .payer(&ctx.accounts.taker)
        .authority(Some(listing))
        .new_owner(&taker)
        .system_program(Some(&ctx.accounts.system_program));

    if let Some(collection) = &ctx.accounts.collection {
        builder.collection(Some(collection));
    }

    builder.invoke_signed(listing_seeds)?;

    // STEP 5 — Mint reward tokens to taker
    let market_name = ctx.accounts.market_place.name.as_str();
    let market_bump = ctx.accounts.market_place.bump;

    let market_seeds: &[&[&[u8]]] = &[&[MARKET_SEED, market_name.as_bytes(), &[market_bump]]];

    // reward = 1 token (with 6 decimals) per 1 SOL spent
    // price is in lamports (1 SOL = 1_000_000_000 lamports)
    // reward_amount = price / 1_000_000_000 * 1_000_000 (6 decimals)
    // simplified: price / 1_000
    let reward_amount = price.checked_div(1_000).unwrap_or(1_000_000); // minimum 1 token if price too small

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.taker_rewards_ata.to_account_info(),
                authority: ctx.accounts.market_place.to_account_info(),
            },
            market_seeds,
        ),
        reward_amount,
    )?;

    msg!("Sold: {}", ctx.accounts.asset.key());
    msg!("Buyer: {}", ctx.accounts.taker.key());
    msg!("Price: {}", price);
    msg!("Fee: {}", fee_amount);
    msg!("Maker received: {}", maker_amount);
    msg!("Rewards minted: {}", reward_amount);

    Ok(())
}
