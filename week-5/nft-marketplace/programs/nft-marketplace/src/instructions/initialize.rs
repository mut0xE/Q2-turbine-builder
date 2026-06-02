use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{
    constants::{MARKET_SEED, REWARDS_SEED, TREASURY_SEED},
    error::MarketPlaceError,
    state::MarketPlace,
};

#[derive(Accounts)]
#[instruction(name:String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        space = MarketPlace::DISCRIMINATOR.len() + MarketPlace::INIT_SPACE,
        payer = admin,
        seeds = [
            MARKET_SEED,
            name.as_str().as_bytes()
        ],
        bump
    )]
    pub market_place: Account<'info, MarketPlace>,

    #[account(
        seeds = [
            TREASURY_SEED,
            admin.key().as_ref()
        ],
        bump
    )]
    pub treasury_pda: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [REWARDS_SEED, market_place.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = market_place
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
    require!(fee > 0, MarketPlaceError::InvalidFee);
    require!(!name.is_empty(), MarketPlaceError::InvalidName);

    ctx.accounts.market_place.set_inner(MarketPlace {
        admin: ctx.accounts.admin.key(),
        fee,
        treasury_bump: ctx.bumps.treasury_pda,
        rewards_bump: ctx.bumps.rewards_mint,
        name,
        bump: ctx.bumps.market_place,
        treasury: ctx.accounts.treasury_pda.key(),
        rewards_mint: ctx.accounts.rewards_mint.key(),
    });
    Ok(())
}
