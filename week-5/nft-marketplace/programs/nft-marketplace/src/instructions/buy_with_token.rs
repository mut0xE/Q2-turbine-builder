use crate::{
    constants::{LISTING_SEED, MARKET_SEED, REWARDS_SEED, TREASURY_SEED},
    error::MarketPlaceError,
    state::{Listing, MarketPlace},
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
    },
};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: just receives tokens
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
        constraint = payment_mint.key() == listing.payment_mint.unwrap()
            @ MarketPlaceError::WrongPaymentMint
    )]
    pub payment_mint: InterfaceAccount<'info, Mint>,

    // Taker's token account for payment_mint
    // Tokens deducted from here
    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    // Maker's token account for payment_mint
    // Tokens received here
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    // Treasury ATA for payment_mint
    // Fee tokens go here
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: InterfaceAccount<'info, TokenAccount>,

    // Treasury PDA — authority for treasury ATA
    #[account(
        mut,
        seeds = [TREASURY_SEED, market_place.admin.as_ref()],
        bump = market_place.treasury_bump
    )]
    pub treasury: SystemAccount<'info>,

    // Reward mint — taker gets rewards after purchase
    #[account(
        mut,
        seeds = [REWARDS_SEED, market_place.key().as_ref()],
        bump = market_place.rewards_bump,
        mint::decimals = 6,
        mint::authority = market_place,
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
        constraint = listing.asset == asset.key()
            @ MarketPlaceError::ListingNotFound,
        constraint = listing.maker == maker.key()
            @ MarketPlaceError::Unauthorized,
        // SPL token listing only
        constraint = listing.payment_mint.is_some()
            @ MarketPlaceError::WrongPaymentMint,
        close = maker,
    )]
    pub listing: Account<'info, Listing>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program
    pub mpl_core_program: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<BuyWithToken>) -> Result<()> {
    let market_place_key = ctx.accounts.market_place.key();
    let asset_key = ctx.accounts.asset.key();
    let listing_bump = ctx.accounts.listing.bump;
    let price = ctx.accounts.listing.price;
    let decimals = ctx.accounts.payment_mint.decimals;

    // STEP 1 — Calculate fee
    let fee_amount = price
        .checked_mul(ctx.accounts.market_place.fee as u64)
        .ok_or(MarketPlaceError::InvalidPrice)?
        .checked_div(10_000)
        .ok_or(MarketPlaceError::InvalidPrice)?;

    let maker_amount = price
        .checked_sub(fee_amount)
        .ok_or(MarketPlaceError::InvalidPrice)?;

    // STEP 2 — Transfer tokens: taker to maker
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.taker_payment_ata.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.maker_payment_ata.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
            },
        ),
        maker_amount,
        decimals,
    )?;

    // STEP 3 — Transfer fee tokens: taker to treasury ATA
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.taker_payment_ata.to_account_info(),
                mint: ctx.accounts.payment_mint.to_account_info(),
                to: ctx.accounts.treasury_payment_ata.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
            },
        ),
        fee_amount,
        decimals,
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
        .payer(&ctx.accounts.taker)
        .authority(Some(listing_info))
        .new_owner(taker_info)
        .system_program(Some(&ctx.accounts.system_program));

    if let Some(collection) = &ctx.accounts.collection {
        builder.collection(Some(collection));
    }

    builder.invoke_signed(listing_seeds)?;

    // STEP 5 — Mint reward tokens to taker
    let market_name = ctx.accounts.market_place.name.clone();
    let market_bump = ctx.accounts.market_place.bump;

    let market_seeds: &[&[&[u8]]] = &[&[MARKET_SEED, market_name.as_bytes(), &[market_bump]]];

    let reward_amount = price.checked_div(1_000).unwrap_or(1_000_000);

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

    msg!("Sold with token: {}", ctx.accounts.asset.key());
    msg!("Buyer: {}", ctx.accounts.taker.key());
    msg!("Price: {}", price);
    msg!("Fee: {}", fee_amount);
    msg!("Rewards minted: {}", reward_amount);

    Ok(())
}
