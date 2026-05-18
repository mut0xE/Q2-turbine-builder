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
pub struct Cancel<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        seeds = [ESCROW_SEED, maker.key().as_ref()],
        bump = escrow.bump,
        constraint = escrow.maker == maker.key() @ EscrowError::Unauthorized,
        constraint = escrow.status == EscrowStatus::Open  @ EscrowError::NotOpen,
        close = maker
    )]
    pub escrow: Account<'info, Escrow>,

    /// CHECK: holds SOL
    #[account(
        mut,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump = escrow.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn cancel_handler(ctx: Context<Cancel>) -> Result<()> {
    let escrow = &ctx.accounts.escrow;

    // Return locked SOL back to Maker
    let escrow_key = escrow.key();
    let vault_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, escrow_key.as_ref(), &[escrow.vault_bump]]];

    let refund_amount = escrow.sol_amount;

    ctx.accounts.escrow.status = EscrowStatus::Cancelled;

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.maker.to_account_info(),
            },
            vault_seeds,
        ),
        refund_amount,
    )?;

    msg!(
        "Escrow cancelled — {} lamports returned to Maker",
        refund_amount
    );
    Ok(())
}
