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

    pub fn initialize(ctx: Context<Initialize>, fee: u16, index: u16) -> Result<()> {
        ctx.accounts.handler(fee, index, &ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount_x: u64, amount_y: u64, min_lp: u64) -> Result<()> {
        ctx.accounts.handler(amount_x, amount_y, min_lp)
    }
}
