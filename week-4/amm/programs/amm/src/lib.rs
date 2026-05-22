use anchor_lang::prelude::*;

declare_id!("9skP2HrosgroRxykvVwF1K4w4FJPTxeuJSpHrgcvRrDK");
mod instructions;
mod state;

use instructions::*;
#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
