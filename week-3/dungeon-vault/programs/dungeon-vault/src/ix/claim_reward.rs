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
#[instruction(dungeon_id:u64)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

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

    /// CHECK: vault PDA only stores SOL
    #[account(
           mut,
            seeds = [
                VAULT_SEED,
                dungeon.key().as_ref()
            ],
            bump = dungeon.vault_bump,
        )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ClaimReward<'info> {
    pub fn handler(&mut self, _dungeon_id: u64) -> Result<()> {
        let dungeon = &mut self.dungeon;
        let player_state = &mut self.player_state;
        let amount = dungeon.amount;

        require!(
            self.vault.lamports() >= amount,
            DungeonError::InsufficientFunds
        );

        require!(
            self.caller.key() == player_state.player,
            DungeonError::Unauthorized
        );

        require!(
            dungeon.status == GameStatus::Finished,
            DungeonError::GameNotFinished
        );

        require!(!dungeon.claimed, DungeonError::AlreadyClaimed);

        if dungeon.alive_players == 0 {
            require!(
                self.caller.key() == dungeon.authority,
                DungeonError::Unauthorized
            );
            msg!("Draw game: creator withdrawing vault");
        } else {
            require!(dungeon.alive_players == 1, DungeonError::Unauthorized);

            require!(player_state.alive, DungeonError::PlayerEliminated);

            require!(
                self.caller.key() == player_state.player,
                DungeonError::Unauthorized
            );

            msg!("Winner claiming reward");
        }

        let dugeon_key = dungeon.key();
        let vault_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, dugeon_key.as_ref(), &[dungeon.vault_bump]]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.caller.to_account_info(),
            },
            vault_seeds,
        );
        transfer(cpi_ctx, amount)?;

        dungeon.claimed = true;
        dungeon.status = GameStatus::Settled;

        emit!(RewardClaimed {
            winner: self.caller.key(),
            amount
        });
        Ok(())
    }
}
