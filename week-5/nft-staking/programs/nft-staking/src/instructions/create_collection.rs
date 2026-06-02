use anchor_lang::prelude::*;
use mpl_core::{
    instructions::CreateCollectionV2CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginAuthorityPair},
    ID as MPL_CORE_ID,
};

use crate::{
    constants::{AUTH_SEED, CONFIG_SEED},
    state::Config,
};
#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // The collection is a brand new MPL Core account.
    // It's NOT a PDA — it's a regular keypair that must be
    // generated fresh and passed as a signer.
    // MPL Core creates the account at this address.
    /// CHECK: created and validated by MPL Core
    #[account(mut)]
    pub collection: Signer<'info>,

    // Config PDA
    // as the update authority of the collection.
    #[account(
           seeds = [CONFIG_SEED,collection.key().as_ref()],
           bump = config.bump,
       )]
    pub config: Account<'info, Config>,

    // CHECK: signing purposes only, derives from correct seeds
    #[account(
        seeds = [AUTH_SEED, collection.key().as_ref()],
        bump,
    )]
    pub update_authority: UncheckedAccount<'info>,

    // MPL Core program
    #[account(address = MPL_CORE_ID)]
    /// CHECK: Program Id of MPL Core
    pub mpl_core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
pub fn handler(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();
    let signers_seeds: &[&[&[u8]]] = &[&[
        AUTH_SEED,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ]];

    CreateCollectionV2CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .collection(&ctx.accounts.collection.to_account_info())
        .payer(&ctx.accounts.payer.to_account_info())
        .update_authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .name(name)
        .uri(uri)
        .plugins(vec![PluginAuthorityPair {
            plugin: Plugin::Attributes(Attributes {
                attribute_list: vec![Attribute {
                    key: "staked_count".to_string(),
                    value: "0".to_string(),
                }],
            }),
            authority: None,
        }])
        .invoke_signed(signers_seeds)?;

    msg!("Collection created: {}", ctx.accounts.collection.key());
    msg!("Update authority: {}", ctx.accounts.update_authority.key());
    msg!("Attributes plugin attached: staked_count = 0");
    Ok(())
}
