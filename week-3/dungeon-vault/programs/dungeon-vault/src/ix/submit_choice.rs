use crate::{constants::*, errors::*, events::*, states::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(dungeon_id:u64)]
pub struct SubmitChoice<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
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
            player.key().as_ref()
        ],
        bump = player_state.bump
    )]
    pub player_state: Account<'info, PlayerState>,
}
impl<'info> SubmitChoice<'info> {
    pub fn handler(&mut self, _dungeon_id: u64, choice: u8) -> Result<()> {
        let dungeon = &self.dungeon;
        let player_state = &mut self.player_state;

        require!(
            dungeon.status == GameStatus::Active,
            DungeonError::GameNotActive
        );

        require!(player_state.alive, DungeonError::PlayerEliminated);

        require!(
            choice >= MIN_CHOICE && choice <= MAX_CHOICE,
            DungeonError::InvalidChoice
        );

        player_state.current_choice = choice;

        emit!(ChoiceSubmitted {
            player: self.player.key(),
            choice,
        });

        Ok(())
    }
}
