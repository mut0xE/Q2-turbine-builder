use anchor_lang::prelude::*;

declare_id!("DUmvpVXXEwrhXW8PDZoX8NapeE97hGPJfyaHnMjZiz7Y");
mod errors;
mod instructions;
mod state;
#[program]
pub mod pay_to_guess {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
