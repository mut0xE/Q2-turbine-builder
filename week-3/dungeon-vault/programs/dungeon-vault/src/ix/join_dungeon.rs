use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{DISCRIMINATOR, DUNGEON_SEED, PLAYER_STATE_SEED, VAULT_SEED},
    errors::DungeonError,
    events::PlayerJoined,
    states::{Dungeon, GameStatus, PlayerState},
};

#[derive(Accounts)]
#[instruction(dungeon_id:u64)]
pub struct JoinDungeon<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

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
               init,
               payer = player,
               space = DISCRIMINATOR + PlayerState::INIT_SPACE,
               seeds = [
                   PLAYER_STATE_SEED,
                   dungeon.key().as_ref(),
                   player.key().as_ref()
               ],
               bump
           )]
    pub player_state: Account<'info, PlayerState>,

    /// CHECK: vault PDA only stores SOL
    #[account(
            mut,
            seeds = [
                VAULT_SEED,
                dungeon.key().as_ref()
            ],
            bump
        )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> JoinDungeon<'info> {
    pub fn handler(&mut self, _dungeon_id: u64, bump: &JoinDungeonBumps) -> Result<()> {
        let dungeon = &mut self.dungeon;

        require!(
            dungeon.status == GameStatus::Waiting,
            DungeonError::GameAlreadyStarted
        );

        require!(
            dungeon.max_players < dungeon.total_players,
            DungeonError::DungeonFull
        );

        self.player_state.set_inner(PlayerState {
            player: self.player.key(),
            dungeon: dungeon.key(),
            alive: true,
            current_choice: 0,
            bump: bump.player_state,
        });

        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.player.to_account_info(),
                    to: self.vault.to_account_info(),
                },
            ),
            dungeon.entry_fee,
        )?;

        dungeon.total_players = dungeon
            .total_players
            .checked_add(1)
            .ok_or(DungeonError::Overflow)?;
        dungeon.alive_players = dungeon
            .alive_players
            .checked_add(1)
            .ok_or(DungeonError::Overflow)?;

        if dungeon.total_players == dungeon.max_players {
            dungeon.status = GameStatus::Active
        }

        emit!(PlayerJoined {
            player: self.player.key(),
            dungeon: dungeon.key(),
        });

        Ok(())
    }
}
