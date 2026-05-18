use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Open,
    Completed,
    Cancelled,
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub maker: Pubkey, // offering SOL, wants USDC
    pub taker: Pubkey,
    pub sol_amount: u64,
    pub usdc_amount: u64,
    pub status: EscrowStatus,
    pub bump: u8,
    pub vault_bump: u8,
}
