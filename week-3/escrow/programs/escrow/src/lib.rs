use anchor_lang::prelude::*;

declare_id!("2sWWAEdyzjhDQH1zyJVvgD6AJdsE1vKGt8xtG6ZxieLx");
mod constants;
mod error;
mod ix;
mod state;

use ix::*;
#[program]
pub mod escrow {
    use super::*;

    /// Maker: lock SOL, state how much USDC want
    pub fn make(ctx: Context<Make>, sol_amount: u64, usdc_amount: u64) -> Result<()> {
        make_handler(ctx, sol_amount, usdc_amount)
    }

    /// Taker: deposit USDC and  get Maker SOL atomically
    pub fn take(ctx: Context<Take>) -> Result<()> {
        take_handler(ctx)
    }

    /// Maker: cancel before anyone takes and get SOL back
    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        cancel_handler(ctx)
    }
}
