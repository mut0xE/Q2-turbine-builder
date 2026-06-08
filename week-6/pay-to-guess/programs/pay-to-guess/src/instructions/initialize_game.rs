use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}
