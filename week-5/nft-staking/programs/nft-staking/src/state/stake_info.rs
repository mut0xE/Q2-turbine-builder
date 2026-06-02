use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct StakeInfo {
    pub owner: Pubkey,      // wallet that staked the NFT
    pub asset: Pubkey,      // MPL Core asset address
    pub collection: Pubkey, // collection the asset belongs to
    pub staked_at: i64,     // unix timestamp when staked
    pub last_claimed: i64,  // unix timestamp of last reward claim
    pub bump: u8,           // bump for this PDA [b"stake", asset, owner]
}
