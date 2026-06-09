use anchor_lang::prelude::*;

use solana_instructions_sysvar::ID as INSTRUCTIONS_SYSVAR_ID;
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};

use crate::{
    constants::{GAME_SEED, PLAYER_SEED, VAULT_SEED},
    errors::GameError,
    state::{GameState, PlayerState},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct GuessPayload {
    pub guess: u8,
}

#[derive(Accounts)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    // PlayerState PDA
    #[account(
        init_if_needed,
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

    ///CHECK: vault holds SOL
    #[account(
        mut,
        seeds = [
            VAULT_SEED,
            game.key().as_ref()
        ],
        bump
    )]
    pub game_vault: SystemAccount<'info>,

    // SYSVAR_INSTRUCTIONS
    /// CHECK: validated by address constraint
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instruction_sysvar: UncheckedAccount<'info>,

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
    require!(sol >= bet_amount, GameError::InsufficientPayment);

    require!(prev_ix.accounts.len() >= 2, GameError::InvalidTransferData);

    require_keys_eq!(
        prev_ix.accounts[0].pubkey,
        ctx.accounts.player.key(),
        GameError::WrongPaymentSource
    );

    require_keys_eq!(
        prev_ix.accounts[1].pubkey,
        ctx.accounts.game_vault.key(),
        GameError::WrongPaymentDestination
    );

    // Payment validated — now check VRF roll is available
    require!(game.roll_ready, GameError::NoRollAvailable);

    let roll = game.current_roll;
    let won = payload.guess == roll;

    game.roll_ready = false;
    game.current_roll = 0;

    let player_state = &mut ctx.accounts.player_state;
    player_state.player = ctx.accounts.player.key();
    player_state.previous_guess = player_state.current_guess;
    player_state.current_guess = payload.guess;
    player_state.current_paid = sol;
    player_state.total_rounds = player_state
        .total_rounds
        .checked_add(1)
        .ok_or(GameError::MathOverflow)?;

    game.total_rounds = game
        .total_rounds
        .checked_add(1)
        .ok_or(GameError::MathOverflow)?;

    msg!(
        "guess={} roll={} bet={} won={}",
        payload.guess,
        roll,
        sol,
        won
    );

    if won {
        let payout = sol.checked_mul(2).ok_or(GameError::MathOverflow)?;
        let vault_balance = ctx.accounts.game_vault.lamports();
        let actual_payout = payout.min(vault_balance);

        let game_key = game.key();
        let seeds = &[VAULT_SEED, game_key.as_ref(), &[game.vault_bump]];
        let signer_seeds = &[&seeds[..]];

        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.game_vault.to_account_info(),
                    to: ctx.accounts.player.to_account_info(),
                },
                signer_seeds,
            ),
            actual_payout,
        )?;

        player_state.total_wins = player_state
            .total_wins
            .checked_add(1)
            .ok_or(GameError::MathOverflow)?;

        let game_cost = actual_payout.saturating_sub(sol);
        game.prize_pool = game.prize_pool.saturating_sub(game_cost);
        msg!("WIN! payout={}", actual_payout);
    } else {
        game.prize_pool = game
            .prize_pool
            .checked_add(sol)
            .ok_or(GameError::MathOverflow)?;
        msg!("LOSE. pool={}", game.prize_pool);
    }

    Ok(())
}

fn decode_transfer_amount(data: &[u8]) -> Result<u64> {
    require!(data.len() >= 12, GameError::InvalidTransferData);
    let disc = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| GameError::InvalidTransferData)?,
    );

    require!(disc == 2, GameError::NotATransfer);

    let lamports = u64::from_le_bytes(
        data[4..12]
            .try_into()
            .map_err(|_| GameError::InvalidTransferData)?,
    );

    Ok(lamports)
}
