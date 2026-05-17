use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Dungeon {
    pub authority: Pubkey,
    pub entry_fee: u64,
    pub dungeon_id: u64,
    pub amount: u64,
    pub total_players: u8,
    pub max_players: u8,
    pub alive_players: u8,
    pub round: u8,
    pub trap_number: u8,
    pub status: GameStatus,
    pub claimed: bool,
    pub dungeon_bump: u8,
    pub vault_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum GameStatus {
    Waiting,
    Active,
    Finished,
    Settled,
}
