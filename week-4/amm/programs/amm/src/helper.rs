use anchor_lang::prelude::*;

#[event]
pub struct DepositEvent {
    pub user: Pubkey,   // who deposited
    pub amount_x: u64,  // how much token X went in
    pub amount_y: u64,  // how much token Y went in
    pub lp_minted: u64, // how many LP tokens were minted
}

#[event]
pub struct SwapEvent {
    pub user: Pubkey,    // who swapped
    pub amount_in: u64,  // tokens sent in
    pub amount_out: u64, // tokens received
    pub fee: u64,        // fee charged
    pub x_to_y: bool,    // direction of swap
}

#[event]
pub struct WithdrawEvent {
    pub user: Pubkey,   // who withdrew
    pub lp_burned: u64, // LP tokens burned
    pub amount_x: u64,  // token X returned
    pub amount_y: u64,  // token Y returned
}
