use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{DUNGEON_SEED, VAULT_SEED},
    errors::DungeonError,
    events::RewardClaimed,
    states::{Dungeon, GameStatus},
};

#[derive(Accounts)]
#[instruction(dungeon_id: u64)]
pub struct ClaimDraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ DungeonError::Unauthorized,
        seeds = [
            DUNGEON_SEED,
            dungeon_id.to_le_bytes().as_ref(),
            authority.key().as_ref()
        ],
        bump = dungeon.dungeon_bump
    )]
    pub dungeon: Account<'info, Dungeon>,

    /// CHECK: vault PDA only stores SOL
    #[account(
        mut,
        seeds = [VAULT_SEED, dungeon.key().as_ref()],
        bump = dungeon.vault_bump
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimDraw<'info> {
    pub fn handler(&mut self, _dungeon_id: u64) -> Result<()> {
        let dungeon = &mut self.dungeon;

        require!(
            dungeon.status == GameStatus::Finished,
            DungeonError::GameNotFinished
        );
        require!(!dungeon.claimed, DungeonError::AlreadyClaimed);
        require!(
            self.vault.lamports() >= dungeon.amount,
            DungeonError::InsufficientFunds
        );
        require!(dungeon.alive_players == 0, DungeonError::Unauthorized);

        msg!("Draw game: creator withdrawing vault");

        let dungeon_key = dungeon.key();
        let vault_seeds: &[&[&[u8]]] =
            &[&[VAULT_SEED, dungeon_key.as_ref(), &[dungeon.vault_bump]]];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.authority.to_account_info(),
                },
                vault_seeds,
            ),
            dungeon.amount,
        )?;

        dungeon.claimed = true;
        dungeon.status = GameStatus::Settled;
        emit!(RewardClaimed {
            winner: self.authority.key(),
            amount: dungeon.amount
        });
        Ok(())
    }
}
