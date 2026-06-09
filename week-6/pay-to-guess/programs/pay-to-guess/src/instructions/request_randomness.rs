use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

use crate::constants::GAME_SEED;
use crate::errors::GameError;
use crate::state::GameState;

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [GAME_SEED, game.authority.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: oracle queue
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_EPHEMERAL_QUEUE)]
    pub oracle_queue: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<RequestRandomness>, client_seed: u8) -> Result<()> {
    require!(!ctx.accounts.game.roll_ready, GameError::RollAlreadyPending);

    let ix = create_request_randomness_ix(RequestRandomnessParams {
        payer: ctx.accounts.payer.key(),
        oracle_queue: ctx.accounts.oracle_queue.key(),
        callback_program_id: crate::ID,
        callback_discriminator: crate::instruction::CallbackRandomness::DISCRIMINATOR.to_vec(),
        caller_seed: [client_seed; 32],
        accounts_metas: Some(vec![SerializableAccountMeta {
            pubkey: ctx.accounts.game.key(),
            is_writable: true,
            is_signer: false,
        }]),
        ..Default::default()
    });

    ctx.accounts
        .invoke_signed_vrf(&ctx.accounts.payer.to_account_info(), &ix)?;
    msg!("Randomness requested");
    Ok(())
}

#[derive(Accounts)]
pub struct CallbackRandomness<'info> {
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,

    #[account(
        mut,
        seeds = [GAME_SEED, game.authority.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, GameState>,
}

pub fn callback_randomness_handler(
    ctx: Context<CallbackRandomness>,
    randomness: [u8; 32],
) -> Result<()> {
    let roll = ephemeral_vrf_sdk::rnd::random_u8_with_range(&randomness, 1, 6);
    msg!("VRF roll: {}", roll);

    ctx.accounts.game.current_roll = roll;
    ctx.accounts.game.roll_ready = true;
    Ok(())
}
