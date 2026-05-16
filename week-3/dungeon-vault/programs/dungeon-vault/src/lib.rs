use anchor_lang::prelude::*;

declare_id!("CMmjB7xBoXeNHjXaLoxzGDiabrXfHNXgaLmQYzL1TBUq");

#[program]
pub mod dungeon_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
