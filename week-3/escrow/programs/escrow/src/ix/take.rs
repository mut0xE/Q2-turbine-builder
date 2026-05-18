use crate::{
    constants::{ESCROW_SEED, VAULT_SEED},
    error::EscrowError,
    state::{Escrow, EscrowStatus},
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// CHECK: Maker wallet — receives USDC
    #[account(
        mut,
        constraint = maker.key() == escrow.maker
    )]
    pub maker: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, escrow.maker.as_ref()],
        bump = escrow.bump,
        constraint = escrow.status == EscrowStatus::Open @ EscrowError::NotOpen,
    )]
    pub escrow: Account<'info, Escrow>,

    /// CHECK: holds SOL
    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump = escrow.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    // USDC mint
    #[account(
        mint::token_program = token_program
    )]
    pub usdc_mint: InterfaceAccount<'info, Mint>,

    // Taker USDC token account (source)
    #[account(
        mut,
        constraint = taker_usdc.mint  == usdc_mint.key(),
        constraint = taker_usdc.owner == taker.key()
    )]
    pub taker_usdc: InterfaceAccount<'info, TokenAccount>,

    // Maker USDC token account (destination)
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = usdc_mint,
        associated_token::authority = maker,
    )]
    pub maker_usdc: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

pub fn take_handler(ctx: Context<Take>) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;

    // Step 1 — Taker sends USDC to Maker
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.taker_usdc.to_account_info(),
                to: ctx.accounts.maker_usdc.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
                mint: ctx.accounts.usdc_mint.to_account_info(),
            },
        ),
        escrow.usdc_amount,
        ctx.accounts.usdc_mint.decimals,
    )?;

    // Step 2 — release vault SOL to Taker
    let escrow_key = escrow.key();
    let vault_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, escrow_key.as_ref(), &[escrow.vault_bump]]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.taker.to_account_info(),
            },
            vault_seeds,
        ),
        escrow.sol_amount,
    )?;

    escrow.taker = ctx.accounts.taker.key();
    escrow.status = EscrowStatus::Completed;

    msg!(
        "Swap complete: {} lamports Taker | {} USDC Maker",
        escrow.sol_amount,
        escrow.usdc_amount
    );
    Ok(())
}
