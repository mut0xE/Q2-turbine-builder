use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::ephem::{FoldableIntentBuilder, MagicIntentBundleBuilder};

use crate::constants::GAME_SEED;
use crate::state::GameState;
#[commit]
#[derive(Accounts)]
pub struct Undelegate<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [GAME_SEED, payer.key().to_bytes().as_slice()], bump)]
    pub game_state: Account<'info, GameState>,
}

pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[ctx.accounts.game_state.to_account_info()])
    .build_and_invoke()?;

    Ok(())
}
