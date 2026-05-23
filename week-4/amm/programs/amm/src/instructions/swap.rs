use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{
    constants::*,
    errors::AmmError,
    helper::{swap_tokens, SwapEvent},
    state::{AmmConfig, Pool},
};

#[derive(Accounts)]
pub struct Swap<'info> {
    // user initiating the swap
    #[account(mut)]
    pub user: Signer<'info>,

    // amm config
    #[account(
            seeds = [SEED_AMM_CONFIG, config.index.to_le_bytes().as_ref()],
            bump = config.bump
        )]
    pub config: Account<'info, AmmConfig>,

    // pool state
    #[account(
        seeds = [
            SEED_POOL,
            pool.config.as_ref(),
            pool.mint_x.as_ref(),
            pool.mint_y.as_ref(),
        ],
        bump = pool.pool_bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    // token X mint
    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    // token Y mint
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [
            SEED_VAULT_X,
            pool.key().as_ref(),
            mint_x.key().as_ref(),

        ],
        bump = pool.vault_x_bump
    )]
    pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [
            SEED_VAULT_Y,
            pool.key().as_ref(),
            mint_y.key().as_ref(),

        ],
        bump = pool.vault_y_bump
    )]
    pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,

    //  lp_provider's token X account
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_ata_x: InterfaceAccount<'info, TokenAccount>,

    //  user's token Y account
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user
    )]
    pub user_ata_y: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    pub fn handler(&mut self, amount_in: u64, min_out: u64, x_to_y: bool) -> Result<()> {
        require!(!self.pool.locked, AmmError::PoolLocked);

        require!(amount_in > 0, AmmError::ZeroAmount);

        let vault_x = self.vault_x.amount;
        let vault_y = self.vault_y.amount;

        let (amount_out, fee) =
            swap_tokens(vault_x, vault_y, amount_in, self.config.fee_rate, x_to_y)?;

        require!(amount_out >= min_out, AmmError::SlippageExceeded);

        // user sends tokens into vault
        self.transfer_to_vault(amount_in, x_to_y)?;

        // vault sends tokens out to user
        self.transfer_from_vault(amount_out, x_to_y)?;

        emit!(SwapEvent {
            user: self.user.key(),
            amount_in,
            amount_out,
            fee,
            x_to_y,
        });

        Ok(())
    }

    // transfer tokens FROM user INTO vault
    fn transfer_to_vault(&mut self, amount: u64, x_to_y: bool) -> Result<()> {
        let (from, to, mint) = if x_to_y {
            (
                self.user_ata_x.to_account_info(),
                self.vault_x.to_account_info(),
                self.mint_x.clone(),
            )
        } else {
            (
                self.user_ata_y.to_account_info(),
                self.vault_y.to_account_info(),
                self.mint_y.clone(),
            )
        };

        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from,
                    mint: mint.to_account_info(),
                    to,
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
            mint.decimals,
        )?;
        Ok(())
    }

    // transfer tokens FROM vault TO user
    fn transfer_from_vault(&mut self, amount: u64, x_to_y: bool) -> Result<()> {
        let (from, to, mint) = if x_to_y {
            (
                self.vault_y.to_account_info(),
                self.user_ata_y.to_account_info(),
                self.mint_y.clone(),
            )
        } else {
            (
                self.vault_x.to_account_info(),
                self.user_ata_x.to_account_info(),
                self.mint_x.clone(),
            )
        };

        let seeds = &[
            SEED_POOL,
            self.pool.config.as_ref(),
            self.pool.mint_x.as_ref(),
            self.pool.mint_y.as_ref(),
            &[self.pool.pool_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from,
                    mint: mint.to_account_info(),
                    to,
                    authority: self.pool.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            mint.decimals,
        )?;
        Ok(())
    }
}
