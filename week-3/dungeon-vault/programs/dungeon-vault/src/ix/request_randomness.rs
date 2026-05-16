use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::{
    anchor::vrf,
    instructions::{create_request_randomness_ix, RequestRandomnessParams},
    types::SerializableAccountMeta,
};

use crate::{
    constants::{DUNGEON_SEED, MAX_CHOICE, PLAYER_STATE_SEED},
    errors::DungeonError,
    instruction::CallbackRandomness,
    states::{Dungeon, GameStatus, PlayerState},
    ID,
};

#[vrf]
#[derive(Accounts)]
#[instruction(dungeon_id:u64)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
           seeds = [
               DUNGEON_SEED,
               dungeon_id.to_le_bytes().as_ref(),
               dungeon.authority.as_ref()
           ],
           bump = dungeon.dungeon_bump
       )]
    pub dungeon: Account<'info, Dungeon>,

    #[account(
    seeds = [
    PLAYER_STATE_SEED,
    dungeon.key().as_ref(),
    payer.key().as_ref()],
    bump
    )
    ]
    pub player_state: Account<'info, PlayerState>,

    /// CHECK: The oracle queue
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CallbackRequestRandomness<'info> {
    /// This check ensure that the vrf_program_identity (which is a PDA) is a singer
    /// enforcing the callback is executed by the VRF program trough CPI
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,

    #[account(mut)]
    pub dungeon: Account<'info, Dungeon>,
}

impl<'info> RequestRandomness<'info> {
    pub fn handler(&mut self, client_seed: u8) -> Result<()> {
        let dungeon = &mut self.dungeon;
        require!(
            dungeon.status == GameStatus::Active,
            DungeonError::GameNotActive
        );

        msg!("Requesting VRF randomness...");

        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.payer.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: ID,
            callback_discriminator: CallbackRandomness::DISCRIMINATOR.to_vec(),
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.dungeon.key(),
                is_signer: false,
                is_writable: true,
            }]),
            caller_seed: [client_seed; 32],
            ..Default::default()
        });
        self.invoke_signed_vrf(&self.payer.to_account_info(), &ix)?;
        Ok(())
    }
}
// Consume Randomness
pub fn callback_randomness_handler(
    ctx: Context<CallbackRequestRandomness>,
    randomness: [u8; 32],
) -> Result<()> {
    let rnd_u8 = ephemeral_vrf_sdk::rnd::random_u8_with_range(&randomness, 1, MAX_CHOICE);
    msg!("Trap number: {:?}", rnd_u8);

    let dungeon = &mut ctx.accounts.dungeon;
    dungeon.trap_number = rnd_u8;
    Ok(())
}
