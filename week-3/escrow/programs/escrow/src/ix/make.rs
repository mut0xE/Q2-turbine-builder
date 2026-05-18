use crate::{
    constants::{ESCROW_SEED, VAULT_SEED},
    error::EscrowError,
    state::{Escrow, EscrowStatus},
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        init,
        payer = maker,
        space = 8 + Escrow::INIT_SPACE,
        seeds = [ESCROW_SEED, maker.key().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    /// CHECK: holds SOL only
    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn make_handler(ctx: Context<Make>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
    require!(sol_amount > 0, EscrowError::InvalidSolAmount);
    require!(usdc_amount > 0, EscrowError::InvalidUsdcAmount);

    ctx.accounts.escrow.set_inner(Escrow {
        maker: ctx.accounts.maker.key(),
        taker: Pubkey::default(),
        sol_amount,
        usdc_amount,
        status: EscrowStatus::Open,
        bump: ctx.bumps.escrow,
        vault_bump: ctx.bumps.vault,
    });

    // Lock maker SOL into vault
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.maker.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        ),
        sol_amount,
    )?;

    msg!(
        "Escrow created: {} lamports for {} USDC",
        sol_amount,
        usdc_amount
    );
    Ok(())
}
