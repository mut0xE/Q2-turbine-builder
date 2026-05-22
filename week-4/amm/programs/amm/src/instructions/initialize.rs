use crate::constants::{
    DISCRIMINATOR, LP_DECIMALS, MAX_FEE, SEED_AMM_CONFIG, SEED_LP_MINT, SEED_POOL, SEED_VAULT_X,
    SEED_VAULT_Y,
};
use crate::errors::AmmError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction(fee: u16)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = DISCRIMINATOR + AmmConfig::INIT_SPACE,
        seeds = [SEED_AMM_CONFIG, &fee.to_le_bytes()],
        bump
    )]
    pub config: Account<'info, AmmConfig>,

    #[account(
            init,
            payer = payer,
            space = 8 + Pool::INIT_SPACE,
            seeds = [
               SEED_POOL,
                config.key().as_ref(),
                mint_x.key().as_ref(),
                mint_y.key().as_ref()
            ],
            bump
        )]
    pub pool: Account<'info, Pool>,

    pub mint_x: InterfaceAccount<'info, Mint>,
    pub mint_y: InterfaceAccount<'info, Mint>,

    #[account(
           init,
           payer = payer,
           mint::decimals = LP_DECIMALS,
           mint::authority = pool,
           seeds = [
               SEED_LP_MINT,
               pool.key().as_ref()
           ],
           bump
       )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = payer,
        token::mint = mint_x,
        token::authority = pool,
        seeds = [
            SEED_VAULT_X,
            pool.key().as_ref(),
            mint_x.key().as_ref()
        ],
        bump
    )]
    pub vault_x: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = mint_y,
        token::authority = pool,
        seeds = [
            SEED_VAULT_Y,
            pool.key().as_ref(),
            mint_y.key().as_ref()
        ],
        bump
    )]
    pub vault_y: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
impl<'info> Initialize<'info> {
    pub fn handler(&mut self, fee: u16, index: u16, bumps: &InitializeBumps) -> Result<()> {
        require!(fee < MAX_FEE, AmmError::InvalidFee);

        self.config.set_inner(AmmConfig {
            fee_rate: fee,
            authority: self.payer.key(),
            index,
            bump: bumps.config,
        });

        self.pool.set_inner(Pool {
            config: self.config.key(),
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            lp_mint: self.lp_mint.key(),
            pool_bump: bumps.pool,
            vault_x_bump: bumps.vault_x,
            vault_y_bump: bumps.vault_y,
            lp_bump: bumps.lp_mint,
        });
        Ok(())
    }
}
