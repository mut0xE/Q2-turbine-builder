use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::accounts::BaseCollectionV1;

use crate::constants::*;
use crate::error::StakingError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    // The admin who sets up the program. Pays for all new accounts.
    #[account(mut)]
    pub admin: Signer<'info>,

    // Config PDA — stores reward_bps, freeze_period, bumps.
    // seeds: [b"config"]
    #[account(
        init,
        payer = admin,
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        seeds = [CONFIG_SEED,collection.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,

    #[account(has_one = update_authority @ StakingError::InvalidUpdateAuthority)]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: signing purposes only, derives from correct seeds
    #[account(
        seeds = [AUTH_SEED, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    // Reward token mint — a PDA so our program is the mint authority.
    // seeds: [b"rewards"]
    // decimals = 6 (like USDC),
    // No one can mint these tokens except our program via CPI.
    #[account(
        init,
        payer = admin,
        seeds = [REWARD_SEED, config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config, // mint authority
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, rewards_bps: u16, freeze_period: u16) -> Result<()> {
    let config = &mut ctx.accounts.config;

    config.set_inner(Config {
        rewards_bps,
        freeze_period,
        rewards_bump: ctx.bumps.reward_mint,
        bump: ctx.bumps.config,
    });
    msg!("Program initialized");
    msg!("Reward rate: {} bps/day", rewards_bps);
    msg!("Freeze period: {} days", freeze_period);
    msg!("Reward mint: {}", ctx.accounts.reward_mint.key());

    Ok(())
}
