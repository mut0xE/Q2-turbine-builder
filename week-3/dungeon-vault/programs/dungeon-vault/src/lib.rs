use anchor_lang::prelude::*;

declare_id!("CuXrhPFnmbt2Ktnpk5RXCR56oLnu9165hyt1zxvCGn7W");
mod constants;
mod errors;
mod events;
mod ix;
mod states;

use ephemeral_rollups_sdk::anchor::ephemeral;
use ix::*;

#[ephemeral]
#[program]
pub mod dungeon_vault {

    use super::*;

    pub fn initialize_dungeon(
        ctx: Context<InitializeDungeon>,
        dungeon_id: u64,
        entry_fee: u64,
        max_players: u8,
    ) -> Result<()> {
        ctx.accounts
            .handler(dungeon_id, entry_fee, max_players, &ctx.bumps)
    }

    pub fn join_dungeon(ctx: Context<JoinDungeon>, dungeon_id: u64) -> Result<()> {
        ctx.accounts.handler(dungeon_id, &ctx.bumps)
    }

    pub fn submit_choice(ctx: Context<SubmitChoice>, dungeon_id: u64, choice: u8) -> Result<()> {
        ctx.accounts.handler(dungeon_id, choice)
    }

    pub fn request_randomness(
        ctx: Context<RequestRandomness>,
        dungeon_id: u64,
        client_seed: u8,
    ) -> Result<()> {
        ctx.accounts.handler(dungeon_id, client_seed)
    }

    pub fn callback_randomness(
        ctx: Context<CallbackRequestRandomness>,
        random_value: [u8; 32],
    ) -> Result<()> {
        request_randomness::callback_randomness_handler(ctx, random_value)
    }

    pub fn resolve_round<'info>(
        ctx: Context<'_, '_, 'info, 'info, ResolveRound<'info>>,
        dungeon_id: u64,
    ) -> Result<()> {
        ctx.accounts.handler(dungeon_id, &ctx.remaining_accounts)
    }

    pub fn claim_winner(ctx: Context<ClaimWinner>, dungeon_id: u64) -> Result<()> {
        ctx.accounts.handler(dungeon_id)
    }
    pub fn claim_draw(ctx: Context<ClaimDraw>, dungeon_id: u64) -> Result<()> {
        ctx.accounts.handler(dungeon_id)
    }

    pub fn delegate_account(ctx: Context<DelegateInput>, account_type: AccountType) -> Result<()> {
        delegate::delegate(ctx, account_type)
    }

    pub fn undelegate<'info>(
        ctx: Context<'_, '_, '_, 'info, Undelegate<'info>>,
        dungeon_id: u64,
    ) -> Result<()> {
        undelegate_handler(ctx, dungeon_id)
    }
}
