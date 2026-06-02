use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{accounts::BaseAssetV1, types::UpdateAuthority};

use crate::{
    constants::{CONFIG_SEED, REWARD_SEED, STAKE_SEED},
    error::StakingError,
    state::{Config, StakeInfo},
};

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    // Owner claims their rewards
    #[account(mut)]
    pub owner: Signer<'info>,

    // The staked asset — verify owner still owns it
    #[account(
        constraint = asset.owner == owner.key()
            @ StakingError::Unauthorized,
        constraint = matches!(
            asset.update_authority,
            UpdateAuthority::Collection(addr) if addr == collection.key()
        ) @ StakingError::WrongCollection,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    /// CHECK: we only need the key for seeds verification
    pub collection: UncheckedAccount<'info>,

    // Config PDA — contains reward_bps and rewards_bump
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    // StakeInfo PDA — contains last_claimed
    // We update last_claimed here but do NOT close it
    // closing only happens in unstake()
    #[account(
        mut,
        seeds = [
            STAKE_SEED,
            asset.key().as_ref(),
            owner.key().as_ref(),
        ],
        bump = stake_info.bump,
        // verify stake_info belongs to this owner + asset
        constraint = stake_info.owner == owner.key()
            @ StakingError::Unauthorized,
        constraint = stake_info.asset == asset.key()
            @ StakingError::NotStaked,
    )]
    pub stake_info: Account<'info, StakeInfo>,

    // Reward token mint — PDA your program controls
    // mint authority = config PDA
    #[account(
        mut,
        seeds = [REWARD_SEED, config.key().as_ref()],
        bump = config.rewards_bump,
        mint::authority = config,
        mint::decimals = 6,
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>,

    // Owner's Associated Token Account for reward tokens
    // Created here if it doesn't exist yet (init_if_needed)
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = reward_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub owner_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimRewards>) -> Result<()> {
    let clock = Clock::get()?;
    let stake_info = &mut ctx.accounts.stake_info;
    let config = &ctx.accounts.config;

    // STEP 1 — Calculate rewards
    //
    // elapsed = seconds since last claim
    // rewards_bps = basis points per day (100 = 1%)
    // formula:
    //   elapsed_days = elapsed / 86400
    //   reward = elapsed_days * rewards_bps / 10_000
    //   scaled by 10^6 (mint decimals)
    //
    // Example:
    //   staked for 1 day, rewards_bps = 100 (1%)
    //   reward = 1 * 100 / 10_000 = 0.01 tokens
    //   with 6 decimals = 10_000 units
    let elapsed = clock
        .unix_timestamp
        .checked_sub(stake_info.last_claimed)
        .ok_or(StakingError::Overflow)? as u64;

    // convert to days (86400 seconds = 1 day)
    let elapsed_days = elapsed.checked_div(86400).ok_or(StakingError::Overflow)?;

    // reward in token units (with 6 decimals)
    let reward_amount = elapsed_days
        .checked_mul(config.rewards_bps as u64)
        .ok_or(StakingError::Overflow)?
        .checked_mul(1_000_000) // 10^6 decimals
        .ok_or(StakingError::Overflow)?
        .checked_div(10_000) // basis points
        .ok_or(StakingError::Overflow)?;

    require!(reward_amount > 0, StakingError::NoRewardsToClaim);

    msg!("Elapsed seconds: {}", elapsed);
    msg!("Elapsed days: {}", elapsed_days);
    msg!("Reward amount: {}", reward_amount);

    // STEP 2 — Mint reward tokens to owner
    //
    // config PDA is the mint authority
    let config_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[config.bump]]];

    mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.reward_mint.to_account_info(),
                to: ctx.accounts.owner_token_account.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            config_seeds,
        ),
        reward_amount,
    )?;

    // STEP 3 — Update last_claimed
    //
    // Reset the timer to now
    // Next claim calculates from this point forward
    // staked_at is NOT touched — freeze_period still counts
    //   from original stake time
    // Asset stays frozen — owner is still staking
    stake_info.last_claimed = clock.unix_timestamp;

    msg!("Rewards claimed: {}", reward_amount);
    msg!("Last claimed updated: {}", clock.unix_timestamp);
    msg!("Asset still staked — not unstaked");

    Ok(())
}
