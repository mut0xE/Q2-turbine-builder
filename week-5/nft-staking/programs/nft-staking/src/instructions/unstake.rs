use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{
        RemovePluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder, UpdatePluginV1CpiBuilder,
    },
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};

use crate::{
    constants::{AUTH_SEED, CONFIG_SEED, STAKE_SEED},
    error::StakingError,
    state::{Config, StakeInfo},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = asset.owner == owner.key()
            @ StakingError::Unauthorized,
        constraint = matches!(
            asset.update_authority,
            UpdateAuthority::Collection(addr) if addr == collection.key()
        ) @ StakingError::WrongCollection,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    // StakeInfo PDA — closed here
    // close = owner means rent lamports go back to owner
    #[account(
        mut,
        seeds = [
            STAKE_SEED,
            asset.key().as_ref(),
            owner.key().as_ref(),
        ],
        bump = stake_info.bump,
        constraint = stake_info.owner == owner.key()
            @ StakingError::Unauthorized,
        constraint = stake_info.asset == asset.key()
            @ StakingError::NotStaked,
        close = owner,
    )]
    pub stake_info: Account<'info, StakeInfo>,

    /// CHECK: PDA signing only
    #[account(
        seeds = [AUTH_SEED, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
    let clock = Clock::get()?;
    let collection_key = ctx.accounts.collection.key();
    let auth_bump = ctx.bumps.update_authority;
    let stake_bump = ctx.accounts.stake_info.bump;

    // STEP 1 — Check freeze_period has passed
    //
    // freeze_period stored in config (in days)
    // staked_at stored in stake_info
    // if not enough time passed → reject
    //
    // Example:
    //   freeze_period = 7 days = 604_800 seconds
    //   staked_at = 1_000_000
    //   now = 1_500_000
    //   elapsed = 500_000 seconds = ~5.7 days
    //   5.7 < 7 = REJECTED
    let elapsed = clock
        .unix_timestamp
        .checked_sub(ctx.accounts.stake_info.staked_at)
        .ok_or(StakingError::Overflow)?;

    let freeze_period_seconds = (ctx.accounts.config.freeze_period as i64)
        .checked_mul(86_400) // days to seconds
        .ok_or(StakingError::Overflow)?;

    require!(
        elapsed >= freeze_period_seconds,
        StakingError::FreezePeriodNotPassed
    );

    // STEP 2 — Remove FreezeDelegate from asset
    //
    // stake_info PDA signs because it's the freeze authority
    // After removal asset is unfrozen
    // Owner can transfer/sell freely again
    let asset_key = ctx.accounts.asset.key();
    let owner_key = ctx.accounts.owner.key();

    let stake_signer_seeds: &[&[&[u8]]] = &[&[
        STAKE_SEED,
        asset_key.as_ref(),
        owner_key.as_ref(),
        &[stake_bump],
    ]];

    RemovePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program)
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner)
        // stake_info PDA signs — it's the FreezeDelegate authority
        .authority(Some(&ctx.accounts.stake_info.to_account_info()))
        .system_program(&ctx.accounts.system_program)
        .plugin_type(PluginType::FreezeDelegate)
        .invoke_signed(stake_signer_seeds)?;

    // STEP 3 — Update Attributes on asset
    //
    // Flip Staked: "true" to "false"
    // update_authority PDA signs
    let auth_signer_seeds: &[&[&[u8]]] = &[&[AUTH_SEED, collection_key.as_ref(), &[auth_bump]]];

    UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program)
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner)
        .authority(Some(&ctx.accounts.update_authority))
        .system_program(&ctx.accounts.system_program)
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: vec![
                Attribute {
                    key: "User".to_string(),
                    value: ctx.accounts.owner.key().to_string(),
                },
                Attribute {
                    key: "Timestamp".to_string(),
                    value: clock.unix_timestamp.to_string(),
                },
                Attribute {
                    key: "Staked".to_string(),
                    value: "false".to_string(),
                },
            ],
        }))
        .invoke_signed(auth_signer_seeds)?;

    // STEP 4 — Decrement staked_count on collection
    let (_, attributes, _) = fetch_plugin::<BaseCollectionV1, Attributes>(
        &ctx.accounts.collection.to_account_info(),
        PluginType::Attributes,
    )?;

    let current_count: u64 = attributes
        .attribute_list
        .iter()
        .find(|a| a.key == "staked_count")
        .and_then(|a| a.value.parse().ok())
        .unwrap_or(0);

    let new_count = current_count.saturating_sub(1);

    UpdateCollectionPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program)
        .collection(&ctx.accounts.collection.to_account_info())
        .payer(&ctx.accounts.owner)
        .authority(Some(&ctx.accounts.update_authority))
        .system_program(&ctx.accounts.system_program)
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: vec![Attribute {
                key: "staked_count".to_string(),
                value: new_count.to_string(),
            }],
        }))
        .invoke_signed(auth_signer_seeds)?;

    msg!("Unstaked asset:  {}", ctx.accounts.asset.key());
    msg!("Owner:           {}", ctx.accounts.owner.key());
    msg!("Staked count:    {}", new_count);
    msg!("Rent returned to owner");

    Ok(())
}
