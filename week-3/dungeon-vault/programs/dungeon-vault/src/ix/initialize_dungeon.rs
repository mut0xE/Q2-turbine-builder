use anchor_lang::prelude::*;

use crate::{
    constants::{DISCRIMINATOR, DUNGEON_SEED, MAX_PLAYERS, MIN_PLAYERS, VAULT_SEED},
    errors::DungeonError,
    events::DungeonInitialized,
    states::{Dungeon, GameStatus},
};

#[derive(Accounts)]
#[instruction(dungeon_id:u64)]
pub struct InitializeDungeon<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = DISCRIMINATOR + Dungeon::INIT_SPACE,
        seeds = [
            DUNGEON_SEED,
            dungeon_id.to_le_bytes().as_ref(),
            creator.key().as_ref()
        ]
        ,bump
    )]
    pub dungeon: Account<'info, Dungeon>,

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

impl<'info> InitializeDungeon<'info> {
    pub fn handler(
        &mut self,
        dungeon_id: u64,
        entry_fee: u64,
        max_players: u8,
        bump: &InitializeDungeonBumps,
    ) -> Result<()> {
        require!(entry_fee > 0, DungeonError::InvalidEntryFee);

        require!(
            max_players >= MIN_PLAYERS && max_players <= MAX_PLAYERS,
            DungeonError::NotEnoughPlayers
        );

        self.dungeon.set_inner(Dungeon {
            authority: self.creator.key(),
            entry_fee,
            dungeon_id,
            total_players: 0,
            max_players,
            alive_players: 0,
            round: 0,
            trap_number: 0,
            status: GameStatus::Waiting,
            dungeon_bump: bump.dungeon,
            vault_bump: bump.vault,
            amount: 0,
            claimed: false,
        });

        emit!(DungeonInitialized {
            authority: self.creator.key(),
            entry_fee,
            max_players,
        });

        Ok(())
    }
}
