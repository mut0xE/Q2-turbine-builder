use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{
        AddPluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder, UpdatePluginV1CpiBuilder,
    },
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType, UpdateAuthority,
    },
    ID as MPL_CORE_ID,
};

use crate::{
    constants::{AUTH_SEED, CONFIG_SEED, STAKE_SEED},
    error::StakingError,
    state::{Config, StakeInfo},
};

#[derive(Accounts)]
pub struct Stake<'info> {
    // Owner must sign because:
    // FreezeDelegate is Owner Managed — owner signature required
    #[account(mut)]
    pub owner: Signer<'info>,

    // The asset being staked
    // Constraint 1: owner field in asset == signer
    // Constraint 2: asset belongs to our collection
    //               via update_authority field
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

    // The collection — we update staked_count here
    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    // Config PDA for this collection pool
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,

    // StakeInfo PDA — created here
    // seeds tie this record to specific asset + owner pair
    // If same owner tries to stake same asset again
    // PDA already exists = init fails = AlreadyStaked
    #[account(
        init,
        payer = owner,
        space = StakeInfo::DISCRIMINATOR.len() + StakeInfo::INIT_SPACE,
        seeds = [
            STAKE_SEED,
            asset.key().as_ref(),
            owner.key().as_ref(),
        ],
        bump,
    )]
    pub stake_info: Account<'info, StakeInfo>,

    // update_authority PDA
    // Signs for:
    //   - Updating Attributes on asset (Staked: true)
    //   - Updating staked_count on collection
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

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    let clock = Clock::get()?;
    let collection_key = ctx.accounts.collection.key();
    let auth_bump = ctx.bumps.update_authority;

    // update_authority PDA signer seeds
    // used for: Attributes update on asset + staked_count on collection
    let auth_signer_seeds: &[&[&[u8]]] = &[&[AUTH_SEED, collection_key.as_ref(), &[auth_bump]]];

    // STEP 1 — Add FreezeDelegate to asset
    //
    // Owner Managed = owner signs (already signing this tx)
    // frozen: true = asset cannot be transferred or sold
    // init_authority = stake_info PDA = only our program
    AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program)
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner)
        .authority(Some(&ctx.accounts.owner)) // owner authorizes
        .system_program(&ctx.accounts.system_program)
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
        // stake_info PDA is freeze authority
        // only removable via unstake() which uses invoke_signed
        .init_authority(PluginAuthority::Address {
            address: ctx.accounts.stake_info.key(),
        })
        .invoke()?;

    // STEP 2 — Update Attributes on asset
    //
    // Attributes already exists from mint_asset()
    // Staked: "false" to "true"
    // update_authority PDA signs = invoke_signed
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
                    value: "true".to_string(),
                },
            ],
        }))
        .invoke_signed(auth_signer_seeds)?;

    // STEP 3 — Save StakeInfo PDA
    //
    // staked_at = now  to used for freeze_period check in unstake
    // last_claimed = now to reward timer starts from stake time
    // bump stored to needed for invoke_signed in unstake/claim
    ctx.accounts.stake_info.set_inner(StakeInfo {
        owner: ctx.accounts.owner.key(),
        asset: ctx.accounts.asset.key(),
        collection: ctx.accounts.collection.key(),
        staked_at: clock.unix_timestamp,
        last_claimed: clock.unix_timestamp,
        bump: ctx.bumps.stake_info,
    });

    // STEP 4 — Increment staked_count on collection Attributes
    //
    // fetch_plugin reads current value
    // parse string to u64 to increment to write full list back
    // update_authority PDA signs
    let (_, attributes, _) = fetch_plugin::<BaseCollectionV1, Attributes>(
        &ctx.accounts.collection.to_account_info(),
        PluginType::Attributes,
    )?;

    // find staked_count, parse it safely
    let current_count: u64 = attributes
        .attribute_list
        .iter()
        .find(|a| a.key == "staked_count")
        .and_then(|a| a.value.parse().ok())
        .unwrap_or(0);

    let new_count = current_count.checked_add(1).ok_or(StakingError::Overflow)?;

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

    msg!("Staked asset:   {}", ctx.accounts.asset.key());
    msg!("Owner:          {}", ctx.accounts.owner.key());
    msg!("Staked at:      {}", clock.unix_timestamp);
    msg!("Staked count:   {}", new_count);

    Ok(())
}
