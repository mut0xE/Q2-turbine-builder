use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, events::*, states::*};

#[derive(Accounts)]
#[instruction(dungeon_id:u64)]

pub struct ResolveRound<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
           mut,
           seeds = [
               DUNGEON_SEED,
               dungeon_id.to_le_bytes().as_ref(),
               dungeon.authority.as_ref()
           ],
           bump = dungeon.dungeon_bump

       )]
    pub dungeon: Account<'info, Dungeon>,

    #[account(
        mut,
        seeds = [
            PLAYER_STATE_SEED,
            dungeon.key().as_ref(),
            player_state.player.as_ref()
        ],
        bump = player_state.bump

    )]
    pub player_state: Account<'info, PlayerState>,
}

impl<'info> ResolveRound<'info> {
    pub fn handler(
        &mut self,

        _dungeon_id: u64,

        remaining_accounts: &'info [AccountInfo<'info>],
    ) -> Result<()> {
        let dungeon = &mut self.dungeon;

        for account_info in remaining_accounts.iter() {
            let mut player_state = Account::<PlayerState>::try_from(account_info)?;

            require!(
                player_state.dungeon == dungeon.key(),
                DungeonError::Unauthorized
            );

            if !player_state.alive {
                continue;
            }

            if player_state.current_choice == dungeon.trap_number {
                player_state.alive = false;

                dungeon.alive_players = dungeon
                    .alive_players
                    .checked_sub(1)
                    .ok_or(DungeonError::Underflow)?;

                emit!(PlayerEliminated {
                    player: player_state.player,
                    round: dungeon.round,
                });
            }
        }

        if dungeon.alive_players == 1 {
            dungeon.status = GameStatus::Finished;
        } else {
            dungeon.round = dungeon.round.checked_add(1).ok_or(DungeonError::Overflow)?;

            dungeon.trap_number = 0;
        }

        Ok(())
    }
}
