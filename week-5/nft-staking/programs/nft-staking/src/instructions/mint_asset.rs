use crate::{
    constants::{AUTH_SEED, CONFIG_SEED},
    state::Config,
};
use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseCollectionV1,
    instructions::CreateV2CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginAuthorityPair},
    ID as MPL_CORE_ID,
};

#[derive(Accounts)]
pub struct MintAsset<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: created and owned by MPL Core
    #[account(mut)]
    pub asset: Signer<'info>,

    // The collection this asset belongs to
    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
          seeds = [CONFIG_SEED, collection.key().as_ref()],
          bump = config.bump,
      )]
    pub config: Account<'info, Config>,

    /// CHECK: signing purposes only, derives from correct seeds
    #[account(
          seeds = [AUTH_SEED, collection.key().as_ref()],
          bump,
      )]
    pub update_authority: UncheckedAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: MPL Core program ID
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();

    // update_authority PDA signs
    let signer_seeds: &[&[&[u8]]] = &[&[
        AUTH_SEED,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ]];

    CreateV2CpiBuilder::new(&ctx.accounts.mpl_core_program)
        // The new asset account
        .asset(&ctx.accounts.asset)
        // Who pays rent for the asset account
        .payer(&ctx.accounts.user)
        // Who owns this NFT after creation
        .owner(Some(&ctx.accounts.user))
        // Link to collection
        // MPL Core writes update_authority = Collection(collection)
        // into the asset account
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        // update_authority of the collection
        .authority(Some(&ctx.accounts.update_authority))
        .system_program(&ctx.accounts.system_program)
        .name(name)
        .uri(uri)
        .plugins(vec![PluginAuthorityPair {
            plugin: Plugin::Attributes(Attributes {
                attribute_list: vec![
                    Attribute {
                        key: "User".to_string(),
                        value: ctx.accounts.user.key().to_string(),
                    },
                    Attribute {
                        key: "Timestamp".to_string(),
                        value: Clock::get()?.unix_timestamp.to_string(),
                    },
                    Attribute {
                        key: "Staked".to_string(),
                        value: "false".to_string(),
                    },
                ],
            }),
            authority: None,
        }])
        .invoke_signed(signer_seeds)?;

    msg!("Asset minted: {}", ctx.accounts.asset.key());
    msg!("Owner: {}", ctx.accounts.user.key());
    msg!("Collection: {}", ctx.accounts.collection.key());
    Ok(())
}
