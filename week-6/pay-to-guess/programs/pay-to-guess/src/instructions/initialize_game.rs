use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{GAME_SEED, VAULT_SEED},
    errors::GameError,
    state::GameState,
};

#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    // One game per authority.
    #[account(
            init,
            payer = authority,
            space = GameState::DISCRIMINATOR.len() + GameState::INIT_SPACE,
            seeds = [GAME_SEED, authority.key().as_ref()],
            bump
        )]
    pub game: Account<'info, GameState>,

    ///CHECK: vault holds SOL
    #[account(
        mut,
        seeds = [
            VAULT_SEED,
            game.key().as_ref()
        ],
        bump
    )]
    pub game_vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
pub fn handler(ctx: Context<InitializeGame>, prize_pool: u64, bet_bps: u16) -> Result<()> {
    require!(prize_pool > 0, GameError::InvalidPrizePool);
    require!(bet_bps <= 10_000, GameError::InvalidBetBps);

    ctx.accounts.game.set_inner(GameState {
        authority: ctx.accounts.authority.key(),
        previous_secret: 0,
        prize_pool,
        bet_amount: 0,
        total_rounds: 0,
        bump: ctx.bumps.game,
        vault_bump: ctx.bumps.game_vault,
        bet_bps,
    });

    // Transfer authority funds into the PDA vault
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.authority.to_account_info(),
            to: ctx.accounts.game_vault.to_account_info(),
        },
    );
    transfer(cpi_ctx, prize_pool)?;
    msg!("Game ready.  pool={}", prize_pool);
    Ok(())
}
