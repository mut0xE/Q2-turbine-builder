use crate::{
    constants::*,
    errors::AmmError,
    state::{AmmConfig, Pool},
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    // must be the current authority
    pub authority: Signer<'info>,

    // config
    #[account(
        mut,
        seeds = [SEED_AMM_CONFIG, config.index.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, AmmConfig>,

    // pool to update locked state
    #[account(
        mut,
        seeds = [
            SEED_POOL,
            pool.config.as_ref(),
            pool.mint_x.as_ref(),
            pool.mint_y.as_ref(),
        ],
        bump = pool.pool_bump,
        constraint = pool.config == config.key() @ AmmError::InvalidConfig
    )]
    pub pool: Account<'info, Pool>,
}

impl<'info> UpdateConfig<'info> {
    pub fn handler(
        &mut self,
        new_fee: Option<u16>,
        locked: Option<bool>,
        new_authority: Option<Pubkey>,
        renounce: bool,
    ) -> Result<()> {
        let current_authority = self.config.authority.ok_or(AmmError::AuthorityRenounced)?;
        require!(
            self.authority.key() == current_authority,
            AmmError::Unauthorized
        );

        if let Some(fee) = new_fee {
            require!(fee < MAX_FEE, AmmError::InvalidFee);
            self.config.fee_rate = fee;
        }

        // lock or unlock pool if provided
        if let Some(lock) = locked {
            self.pool.locked = lock;
        }

        if renounce {
            self.config.authority = None;
            return Ok(());
        }

        // transfer authority to new address
        if let Some(new_auth) = new_authority {
            self.config.authority = Some(new_auth);
        }

        Ok(())
    }
}
