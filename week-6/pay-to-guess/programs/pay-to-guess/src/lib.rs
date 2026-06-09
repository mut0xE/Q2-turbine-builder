use anchor_lang::prelude::*;

declare_id!("452bo1TyLVXwB9hrcwNGQRxk23yB8nc6LAVHqspY3Dv5");
mod constants;
mod errors;
mod instructions;
mod state;
use ephemeral_rollups_sdk::anchor::ephemeral;
use instructions::*;

#[ephemeral]
#[program]
pub mod pay_to_guess {

    use super::*;

    pub fn initialize_game(
        ctx: Context<InitializeGame>,
        prize_pool: u64,
        bet_bps: u16,
    ) -> Result<()> {
        initialize_game::handler(ctx, prize_pool, bet_bps)
    }

    // pub fn initialize_player(ctx: Context<InitializePlayer>) -> Result<()> {
    //     initialize_player::handler(ctx)
    // }

    pub fn play(ctx: Context<Play>, payload: play::GuessPayload) -> Result<()> {
        play::handler(ctx, payload)
    }
    pub fn request_randomness(ctx: Context<RequestRandomness>, client_seed: u8) -> Result<()> {
        request_randomness::handler(ctx, client_seed)
    }
    pub fn callback_randomness(
        ctx: Context<CallbackRandomness>,
        randomness: [u8; 32],
    ) -> Result<()> {
        callback_randomness_handler(ctx, randomness)
    }

    pub fn close_game(ctx: Context<CloseGame>) -> Result<()> {
        close_game::handler(ctx)
    }

    pub fn delegate_account(ctx: Context<DelegateInput>, account_type: AccountType) -> Result<()> {
        delegate::delegate(ctx, account_type)
    }
    pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
        undelegate::undelegate(ctx)
    }
}
