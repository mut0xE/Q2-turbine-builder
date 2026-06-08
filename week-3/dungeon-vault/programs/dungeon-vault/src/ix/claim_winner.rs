use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{DUNGEON_SEED, PLAYER_STATE_SEED, VAULT_SEED},
    errors::DungeonError,
    events::RewardClaimed,
    states::{Dungeon, GameStatus, PlayerState},
};

#[derive(Accounts)]
pub struct ClaimWinner<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [
            DUNGEON_SEED,
            dungeon.dungeon_id.to_le_bytes().as_ref(),
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
            caller.key().as_ref()
        ],
        bump = player_state.bump
    )]
    pub player_state: Account<'info, PlayerState>,

    /// CHECK: vault PDA only stores SOL
    #[account(
        mut,
        seeds = [VAULT_SEED, dungeon.key().as_ref()],
        bump = dungeon.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimWinner<'info> {
    pub fn handler(&mut self, _dungeon_id: u64) -> Result<()> {
        let dungeon = &mut self.dungeon;
        let player_state = &self.player_state;

        require!(
            dungeon.status == GameStatus::Finished,
            DungeonError::GameNotFinished
        );
        require!(!dungeon.claimed, DungeonError::AlreadyClaimed);
        require!(
            self.vault.lamports() >= dungeon.amount,
            DungeonError::InsufficientFunds
        );
        require!(dungeon.alive_players == 1, DungeonError::Unauthorized);
        require!(player_state.alive, DungeonError::PlayerEliminated);

        let dungeon_key = dungeon.key();
        let vault_seeds: &[&[&[u8]]] =
            &[&[VAULT_SEED, dungeon_key.as_ref(), &[dungeon.vault_bump]]];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.caller.to_account_info(),
                },
                vault_seeds,
            ),
            dungeon.amount,
        )?;

        dungeon.claimed = true;
        dungeon.status = GameStatus::Settled;
        emit!(RewardClaimed {
            winner: self.caller.key(),
            amount: dungeon.amount
        });
        Ok(())
    }
}
