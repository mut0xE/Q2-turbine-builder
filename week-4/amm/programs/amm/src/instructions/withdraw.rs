use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        burn, transfer_checked, Burn, Mint, TokenAccount, TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::*,
    errors::AmmError,
    helper::{calculate_withdraw, WithdrawEvent},
    state::Pool,
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    // user removing liquidity
    #[account(mut)]
    pub user: Signer<'info>,

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
    #[account(
            constraint = mint_x.key() == pool.mint_x @ AmmError::InvalidMint
        )]
    pub mint_x: InterfaceAccount<'info, Mint>,

    // token Y mint
    #[account(
           constraint = mint_y.key() == pool.mint_y @ AmmError::InvalidMint
       )]
    pub mint_y: InterfaceAccount<'info, Mint>,

    // LP mint
    #[account(
        mut,
        seeds = [SEED_LP_MINT, pool.key().as_ref()],
        bump = pool.lp_bump,
        constraint = lp_mint.key() == pool.lp_mint @ AmmError::InvalidMint
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    // vault X
    #[account(
        mut,
        seeds = [
            SEED_VAULT_X,
            pool.key().as_ref(),
            mint_x.key().as_ref(),
        ],
        bump = pool.vault_x_bump,
        constraint = vault_x.owner == pool.key() @ AmmError::InvalidVaultOwner,
        constraint = vault_x.mint == mint_x.key() @ AmmError::InvalidMint
    )]
    pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,

    // vault Y
    #[account(
        mut,
        seeds = [
            SEED_VAULT_Y,
            pool.key().as_ref(),
            mint_y.key().as_ref(),
        ],
        bump = pool.vault_y_bump,
        constraint = vault_y.owner == pool.key() @ AmmError::InvalidVaultOwner,
        constraint = vault_y.mint == mint_y.key() @ AmmError::InvalidMint
    )]
    pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,

    // user's token X account
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user
    )]
    pub user_ata_x: Box<InterfaceAccount<'info, TokenAccount>>,

    // user's token Y account
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user
    )]
    pub user_ata_y: Box<InterfaceAccount<'info, TokenAccount>>,

    // user's LP account
    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = user
    )]
    pub user_lp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
impl<'info> Withdraw<'info> {
    pub fn handler(&mut self, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        require!(lp_amount > 0, AmmError::ZeroLpAmount);

        let vault_x = self.vault_x.amount;
        let vault_y = self.vault_y.amount;
        let lp_supply = self.lp_mint.supply;

        let (x_out, y_out) = calculate_withdraw(vault_x, vault_y, lp_supply, lp_amount)?;

        require!(x_out >= min_x, AmmError::SlippageExceeded);
        require!(y_out >= min_y, AmmError::SlippageExceeded);

        // burn LP tokens first
        self.burn_lp(lp_amount)?;

        // send X from vault to user
        self.withdraw_token(x_out, true)?;

        // send Y from vault to user
        self.withdraw_token(y_out, false)?;

        emit!(WithdrawEvent {
            user: self.user.key(),
            lp_burned: lp_amount,
            amount_x: x_out,
            amount_y: y_out,
        });

        Ok(())
    }
    // burn LP tokens from user's LP account
    // user signs — they own the LP tokens
    fn burn_lp(&mut self, lp_amount: u64) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.lp_mint.to_account_info(),
                from: self.user_lp_ata.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );
        burn(cpi_ctx, lp_amount)?;
        Ok(())
    }

    // transfer tokens from vault back to user
    fn withdraw_token(&mut self, amount: u64, is_x: bool) -> Result<()> {
        let (from, to, mint) = if is_x {
            (
                self.vault_x.to_account_info(),
                self.user_ata_x.to_account_info(),
                self.mint_x.clone(),
            )
        } else {
            (
                self.vault_y.to_account_info(),
                self.user_ata_y.to_account_info(),
                self.mint_y.clone(),
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

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from,
                mint: mint.to_account_info(),
                to,
                authority: self.pool.to_account_info(),
            },
            signer_seeds,
        );
        transfer_checked(cpi_ctx, amount, mint.decimals)?;
        Ok(())
    }
}
