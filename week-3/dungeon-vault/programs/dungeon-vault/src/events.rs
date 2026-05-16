use anchor_lang::prelude::*;

#[event]
pub struct DungeonInitialized {
    pub authority: Pubkey,
    pub entry_fee: u64,
    pub max_players: u8,
}

#[event]
pub struct PlayerJoined {
    pub player: Pubkey,
    pub dungeon: Pubkey,
}

#[event]
pub struct ChoiceSubmitted {
    pub player: Pubkey,
    pub choice: u8,
}

#[event]
pub struct RandomnessFulfilled {
    pub dungeon: Pubkey,
    pub random_value: u64,
    pub trap_number: u8,
}

#[event]
pub struct PlayerEliminated {
    pub player: Pubkey,
    pub round: u8,
}

#[event]
pub struct RewardClaimed {
    pub winner: Pubkey,
    pub amount: u64,
}
