use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
    },
};

use crate::{
    constants::*,
    errors::AmmError,
    helper::{lp_tokens, DepositEvent},
    state::Pool,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    // the user paying for the deposit
    #[account(mut)]
    pub lp_provider: Signer<'info>,

    // pool state
    #[account(
        mut,
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

    // LP mint
    #[account(
        mut,
        seeds = [
            SEED_LP_MINT,
            pool.key().as_ref()
        ],
        bump = pool.lp_bump
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,

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
        associated_token::authority = lp_provider
    )]
    pub lp_provider_ata_x: InterfaceAccount<'info, TokenAccount>,

    //  lp_provider's token Y account
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = lp_provider
    )]
    pub lp_provider_ata_y: InterfaceAccount<'info, TokenAccount>,

    // lp_provider's LP account
    #[account(
            init_if_needed,
            payer = lp_provider,
            associated_token::mint = lp_mint,
            associated_token::authority = lp_provider
        )]
    pub lp_provider_lp_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn handler(&mut self, amount_x: u64, amount_y: u64, min_lp: u64) -> Result<()> {
        require!(!self.pool.locked, AmmError::PoolLocked);

        require!(amount_x > 0 && amount_y > 0, AmmError::ZeroAmount);

        let vault_x = self.vault_x.amount;
        let vault_y = self.vault_y.amount;
        let lp_supply = self.lp_mint.supply;

        let lp_to_mint = lp_tokens(vault_x, vault_y, lp_supply, amount_x, amount_y)?;

        require!(lp_to_mint >= min_lp, AmmError::SlippageExceeded);

        // transfer X into vault
        self.deposit_token(true, amount_x)?;
        // transfer Y into vault
        self.deposit_token(false, amount_y)?;

        // mint LP to user
        self.mint_lp(lp_to_mint)?;

        // emit event
        emit!(DepositEvent {
            user: self.lp_provider.key(),
            amount_x,
            amount_y,
            lp_minted: lp_to_mint,
        });
        Ok(())
    }

    // transfer token X or Y into the vault
    // is_x: true = transfer X, false = transfer Y
    fn deposit_token(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let token_program = self.token_program.to_account_info();

        let (from, to, mint) = if is_x {
            (
                self.lp_provider_ata_x.to_account_info(),
                self.vault_x.to_account_info(),
                self.mint_x.clone(),
            )
        } else {
            (
                self.lp_provider_ata_y.to_account_info(),
                self.vault_y.to_account_info(),
                self.mint_y.clone(),
            )
        };

        let cpi_ctx = CpiContext::new(
            token_program,
            TransferChecked {
                from,
                mint: mint.to_account_info(),
                to,
                authority: self.lp_provider.to_account_info(),
            },
        );
        transfer_checked(cpi_ctx, amount, mint.decimals)?;
        Ok(())
    }

    // mint LP tokens to user
    // pool PDA signs because it is the mint authority
    fn mint_lp(&mut self, amount: u64) -> Result<()> {
        let pool_key = self.pool.key();

        let signer_seeds: &[&[&[u8]]] = &[&[SEED_POOL, pool_key.as_ref(), &[self.pool.pool_bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.lp_mint.to_account_info(),
                to: self.lp_provider_lp_ata.to_account_info(),
                authority: self.pool.to_account_info(),
            },
            signer_seeds,
        );

        mint_to(cpi_ctx, amount)?;
        Ok(())
    }
}
