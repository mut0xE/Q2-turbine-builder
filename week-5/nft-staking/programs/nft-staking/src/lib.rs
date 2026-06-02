use anchor_lang::prelude::*;

declare_id!("2Y9zrc8BhfB5A2dDZBn98BmPc8AK7G33vDUxScfVRrMf");
mod constants;
mod error;
mod instructions;
mod state;

use instructions::*;
#[program]
pub mod nft_staking {

    use crate::instructions::{create_collection, mint_asset};

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        rewards_bps: u16,
        freeze_period: u16,
    ) -> Result<()> {
        initialize::handler(ctx, rewards_bps, freeze_period)
    }

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        name: String,
        uri: String,
    ) -> Result<()> {
        create_collection::handler(ctx, name, uri)
    }

    pub fn mint_asset(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
        mint_asset::handler(ctx, name, uri)
    }
}
