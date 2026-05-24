use anchor_lang::prelude::*;

declare_id!("9skP2HrosgroRxykvVwF1K4w4FJPTxeuJSpHrgcvRrDK");
mod constants;
mod errors;
mod helper;
mod instructions;
mod state;
use instructions::*;
#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, index: u64, fee: Option<u16>) -> Result<()> {
        ctx.accounts.handler(index, fee, &ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount_x: u64, amount_y: u64, min_lp: u64) -> Result<()> {
        ctx.accounts.handler(amount_x, amount_y, min_lp)
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, min_out: u64, x_to_y: bool) -> Result<()> {
        ctx.accounts.handler(amount_in, min_out, x_to_y)
    }

    pub fn withdraw(ctx: Context<Withdraw>, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        ctx.accounts.handler(lp_amount, min_x, min_y)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_fee: Option<u16>,
        locked: Option<bool>,
        new_authority: Option<Pubkey>,
        renounce: bool,
    ) -> Result<()> {
        ctx.accounts
            .handler(new_fee, locked, new_authority, renounce)
    }
}
