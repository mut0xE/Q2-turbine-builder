use anchor_lang::prelude::*;

declare_id!("2Y9zrc8BhfB5A2dDZBn98BmPc8AK7G33vDUxScfVRrMf");
mod constants;
mod error;
mod instructions;
mod state;

use instructions::*;
#[program]
pub mod nft_staking {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        rewards_bps: u16,
        freeze_period: u16,
    ) -> Result<()> {
        initialize::handler(ctx, rewards_bps, freeze_period)
    }
}
