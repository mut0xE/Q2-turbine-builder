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
}
