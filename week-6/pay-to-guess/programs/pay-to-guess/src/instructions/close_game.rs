use crate::{
    constants::{GAME_SEED, VAULT_SEED},
    errors::GameError,
    state::GameState,
};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct CloseGame<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    // Close game state — rent goes back to authority
    #[account(
        mut,
        seeds = [GAME_SEED, authority.key().as_ref()],
        bump = game.bump,
        has_one = authority @ GameError::Unauthorized,
        close = authority
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: vault PDA
    #[account(
        mut,
        seeds = [VAULT_SEED, game.key().as_ref()],
        bump = game.vault_bump
    )]
    pub game_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CloseGame>) -> Result<()> {
    let vault_balance = ctx.accounts.game_vault.lamports();

    if vault_balance > 0 {
        let game_key = ctx.accounts.game.key();
        let seeds = &[
            VAULT_SEED,
            game_key.as_ref(),
            &[ctx.accounts.game.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.key(),
                Transfer {
                    from: ctx.accounts.game_vault.to_account_info(),
                    to: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds,
            ),
            vault_balance,
        )?;
    }

    msg!(
        "Game closed. rounds={} vault_drained={}",
        ctx.accounts.game.total_rounds,
        vault_balance
    );

    Ok(())
}
