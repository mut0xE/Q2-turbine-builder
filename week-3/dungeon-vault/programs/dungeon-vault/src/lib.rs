use anchor_lang::prelude::*;

declare_id!("CMmjB7xBoXeNHjXaLoxzGDiabrXfHNXgaLmQYzL1TBUq");
mod constants;
mod errors;
mod events;
mod ix;
mod states;
mod utils;

use ix::*;
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

    pub fn request_randomness(ctx: Context<RequestRandomness>, client_seed: u8) -> Result<()> {
        ctx.accounts.handler(client_seed)
    }

    pub fn callback_randomness(
        ctx: Context<CallbackRequestRandomness>,
        random_value: [u8; 32],
    ) -> Result<()> {
        request_randomness::callback_randomness_handler(ctx, random_value)
    }
}
