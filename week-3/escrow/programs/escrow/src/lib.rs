use anchor_lang::prelude::*;

declare_id!("2sWWAEdyzjhDQH1zyJVvgD6AJdsE1vKGt8xtG6ZxieLx");
mod state;

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
