use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::{anchor::commit, ephem::MagicIntentBundleBuilder};

use crate::{
    constants::DUNGEON_SEED,
    errors::DungeonError,
    states::{Dungeon, GameStatus},
};

#[commit]
#[derive(Accounts)]
#[instruction(dungeon_id: u64)]
pub struct Undelegate<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        seeds = [
            DUNGEON_SEED,
            &dungeon_id.to_le_bytes(),
            dungeon.authority.as_ref()
        ],
        bump = dungeon.dungeon_bump,
        constraint = dungeon.status == GameStatus::Finished @ DungeonError::GameNotFinished
    )]
    pub dungeon: Account<'info, Dungeon>,
}

pub fn undelegate_handler<'info>(
    ctx: Context<'_, '_, '_, 'info, Undelegate<'info>>,
    _dungeon_id: u64,
) -> Result<()> {
    let mut accounts_to_undelegate: Vec<AccountInfo<'info>> =
        vec![ctx.accounts.dungeon.to_account_info()];

    for account_info in ctx.remaining_accounts.iter() {
        accounts_to_undelegate.push(account_info.clone());
    }

    MagicIntentBundleBuilder::new(
        ctx.accounts.player.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&accounts_to_undelegate)
    .build_and_invoke()?;
    Ok(())
}
