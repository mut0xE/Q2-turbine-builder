use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::vrf;

use crate::{
    constants::{GAME_SEED, PLAYER_SEED},
    errors::GameError,
    state::{GameState, PlayerState},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GuessPayload {
    pub guess: u8,
    pub word: String,
}

#[vrf]
#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    // PlayerState PDA
    #[account(
        init,
        payer = player,
        space = PlayerState::DISCRIMINATOR.len() + PlayerState::INIT_SPACE,
        seeds = [PLAYER_SEED, player.key().as_ref()],
        bump
    )]
    pub player_state: Account<'info, PlayerState>,

    #[account(
        mut,
        seeds = [GAME_SEED, game.authority.as_ref()],
        bump = game.bump
    )]
    pub game: Account<'info, GameState>,

    /// CHECK: The oracle queue
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_EPHEMERAL_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,

    // SYSVAR_INSTRUCTIONS
    /// CHECK: validated by address constraint
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instruction_sysvar: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Play>, payload: GuessPayload) -> Result<()> {
    require!(
        payload.guess >= 1 && payload.guess <= 6,
        GameError::InvalidGuess
    );

    let game = &mut ctx.accounts.game;
    let bet_amount = game
        .prize_pool
        .checked_mul(game.bet_bps as u64)
        .ok_or(GameError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(GameError::MathOverflow)?;

    require!(bet_amount > 0, GameError::InvalidBetAmount);

    // ── INSTRUCTION INTROSPECTION ────────────────────
    //
    // Player sends ONE transaction with TWO instructions:
    //
    //   ix[n-1]  →  System::Transfer  (player pays bet_amount to vault)
    //   ix[n]    →  play(GuessPayload) (this instruction)
    //
    // We read ix[n-1] at runtime to verify payment happened.
    // If it didn't whole tx reverts atomically.

    let idx = load_current_index_checked(&ctx.accounts.instruction_sysvar.to_account_info())
        .map_err(|_| GameError::IntrospectionFailed)?;

    msg!("play() at tx index {}", idx);

    // Must have a previous instruction
    require!(idx > 0, GameError::MissingPaymentInstruction);

    let prev_ix = load_instruction_at_checked(
        (idx - 1) as usize,
        &ctx.accounts.instruction_sysvar.to_account_info(),
    )
    .map_err(|_| GameError::MissingPaymentInstruction)?;

    require!(
        prev_ix.program_id == system_program::ID,
        GameError::PaymentNotSystemProgram
    );

    // Decode the transfer amount.
    // System Program => Transfer: ix 2
    // Transfer { lamports: u64 },
    //   bytes [0..4]  = discriminator u32 LE = 2
    //   bytes [4..12] = lamports u64 LE

    let sol = decode_transfer_amount(&prev_ix.data)?;
    require!(paid >= bet_amount, GameError::InsufficientPayment);

    require_keys_eq!(
        prev_ix.accounts[1].pubkey,
        ctx.accounts.game_vault.key(),
        GameError::WrongPaymentDestination
    );

    ctx.accounts.player_state.set_inner(PlayerState {
        player: ctx.accounts.player.key(),
        guess: payload.guess,
        paid: sol,
        bump: ctx.bumps.player_state,
    });

    game.total_rounds = game
        .total_rounds
        .checked_add(1)
        .ok_or(GameError::MathOverflow)?;
    Ok(())
}

fn decode_transfer_amount(data: &[u8]) -> Result<u64> {
    require!(data.len() >= 12, GameError::BadTransferData);
    let disc = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| GameError::InvalidTransferData)?,
    );

    require!(disc == 2, GameError::NotATransfer);

    let lamports = u64::from_le_bytes(
        data[4..12]
            .try_into()
            .map_err(|_| GameError::BadTransferData)?,
    );

    Ok(lamports)
}

pub fn request_randomness(ctx: Context<Play>, client_seed: u8) -> Result<()> {
    msg!("Requesting VRF randomness...");

    let ix = create_request_randomness_ix(RequestRandomnessParams {
        payer: ctx.accounts.player.key(),
        oracle_queue: ctx.accounts.oracle_queue.key(),
        callback_program_id: crate::ID,
        callback_discriminator: instruction::CallbackPlay::DISCRIMINATOR.to_vec(),
        caller_seed: client_seed,
        accounts_metas: Some(vec![
            SerializableAccountMeta {
                pubkey: ctx.accounts.player_state.key(),
                is_writable: true,
                is_signer: false,
            },
            SerializableAccountMeta {
                pubkey: ctx.accounts.game.key(),
                is_writable: true,
                is_signer: false,
            },
            SerializableAccountMeta {
                pubkey: ctx.accounts.game_vault.key(),
                is_writable: true,
                is_signer: false,
            },
            SerializableAccountMeta {
                pubkey: ctx.accounts.player.key(),
                is_writable: true,
                is_signer: false,
            },
        ]),
        ..Default::default()
    });

    ctx.accounts
        .invoke_signed_vrf(&ctx.accounts.payer.to_account_info(), &ix)?;
    msg!("Randomness requested ");
    Ok(())
}
