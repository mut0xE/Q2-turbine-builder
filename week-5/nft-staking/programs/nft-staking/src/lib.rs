use anchor_lang::prelude::*;

declare_id!("2Y9zrc8BhfB5A2dDZBn98BmPc8AK7G33vDUxScfVRrMf");

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
