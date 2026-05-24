use anchor_lang::prelude::*;

use crate::{constants::FEE_DENOMINATOR, errors::AmmError};

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

pub fn calculate_initial_lp_tokens(amount_x: u64, amount_y: u64) -> Result<u64> {
    let lp = integer_sqrt(
        (amount_x as u128)
            .checked_mul(amount_y as u128)
            .ok_or(AmmError::MathOverflow)?,
    ) as u64;

    require!(lp > 0, AmmError::ZeroLpAmount);
    Ok(lp)
}

fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Takes current pool state + user deposit amounts
/// Returns how many LP tokens to mint to the user
// vault_x  — current token X reserve in pool
// vault_y  — current token Y reserve in pool
// lp_supply — total LP tokens
// amount_x — token X user wants to deposit
// amount_y — token Y user wants to deposit
pub fn lp_tokens(
    vault_x: u64,
    vault_y: u64,
    lp_supply: u64,
    amount_x: u64,
    amount_y: u64,
) -> Result<u64> {
    // first deposit, pool is empty
    if lp_supply == 0 {
        calculate_initial_lp_tokens(amount_x, amount_y)
    } else {
        calculate_deposit_lp(vault_x, vault_y, lp_supply, amount_x, amount_y)
    }
}

pub fn calculate_deposit_lp(
    vault_x: u64,
    vault_y: u64,
    lp_supply: u64,
    amount_x: u64,
    amount_y: u64,
) -> Result<u64> {
    // lp_x = how much LP the X contribution earns
    // formula: dx * LP / X
    let lp_x = (amount_x as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(vault_x as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    // lp_y = how much LP the Y contribution earns
    // formula: dy * LP / Y
    let lp_y = (amount_y as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(vault_y as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    let lp = lp_x.min(lp_y);

    require!(lp > 0, AmmError::ZeroLpAmount);
    Ok(lp)
}

/// calculate swap output using x*y=k invariant
/// when user swaps X in, they get Y out
/// dy = Y * dx' / (X + dx')
pub fn calculate_swap(
    vault_in: u64,
    vault_out: u64,
    amount_in: u64,
    fee_rate: u16,
) -> Result<(u64, u64)> {
    require!(amount_in > 0, AmmError::ZeroAmount);
    require!(vault_in > 0 && vault_out > 0, AmmError::ZeroAmount);

    // deduct fee from input
    let amount_in_after_fee = (amount_in as u128)
        .checked_mul((FEE_DENOMINATOR - fee_rate as u64) as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(FEE_DENOMINATOR as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    // fee stays in vault
    // that growth is the LP reward
    let fee = amount_in
        .checked_sub(amount_in_after_fee)
        .ok_or(AmmError::MathOverflow)?;

    // calculate output using x * y = k
    let amount_out = (vault_out as u128)
        .checked_mul(amount_in_after_fee as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(
            (vault_in as u128)
                .checked_add(amount_in_after_fee as u128)
                .ok_or(AmmError::MathOverflow)?,
        )
        .ok_or(AmmError::MathOverflow)? as u64;

    require!(amount_out > 0, AmmError::ZeroAmount);

    Ok((amount_out, fee))
}

/// x_to_y: true  = user sends X gets Y
/// x_to_y: false = user sends Y gets X
pub fn swap_tokens(
    vault_x: u64,
    vault_y: u64,
    amount_in: u64,
    fee_rate: u16,
    x_to_y: bool,
) -> Result<(u64, u64)> {
    if x_to_y {
        // user sends X to vault_x is in, vault_y is out
        calculate_swap(vault_x, vault_y, amount_in, fee_rate)
    } else {
        // user sends Y to vault_y is in, vault_x is out
        calculate_swap(vault_y, vault_x, amount_in, fee_rate)
    }
}

/// calculate how much X and Y to return when user burns LP tokens
/// x_out = X * lp_amount / lp_supply
/// y_out = Y * lp_amount / lp_supply
pub fn calculate_withdraw(
    vault_x: u64,
    vault_y: u64,
    lp_supply: u64,
    lp_amount: u64,
) -> Result<(u64, u64)> {
    require!(lp_amount > 0, AmmError::ZeroLpAmount);
    require!(lp_supply > 0, AmmError::ZeroLpAmount);
    require!(lp_amount <= lp_supply, AmmError::MathOverflow);

    let x_out = (vault_x as u128)
        .checked_mul(lp_amount as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    let y_out = (vault_y as u128)
        .checked_mul(lp_amount as u128)
        .ok_or(AmmError::MathOverflow)?
        .checked_div(lp_supply as u128)
        .ok_or(AmmError::MathOverflow)? as u64;

    require!(x_out > 0 && y_out > 0, AmmError::ZeroAmount);

    Ok((x_out, y_out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_lp_equal_amounts() {
        let result = calculate_initial_lp_tokens(100, 100).unwrap();
        assert_eq!(result, 100)
    }
    #[test]
    fn test_initial_lp_unequal_amounts() {
        let result = calculate_initial_lp_tokens(100, 400).unwrap();
        assert_eq!(result, 200)
    }

    #[test]
    fn test_initial_lp_zero_x_fails() {
        assert!(calculate_initial_lp_tokens(0, 400).is_err())
    }

    #[test]
    fn test_initial_lp_zero_y_fails() {
        assert!(calculate_initial_lp_tokens(100, 0).is_err());
    }

    /*
    // pool: 100 X, 400 Y, 200 LP
    // user: 10 X, 40 Y
    // lp_x = 10 * 200 / 100 = 20
    // lp_y = 40 * 200 / 400 = 20
    // lp   = min(20,20) = 20
     */
    #[test]
    fn test_deposit_lp_correct_ratio() {
        let result = calculate_deposit_lp(100, 400, 200, 10, 40).unwrap();
        assert_eq!(result, 20);
    }

    /*
    // user deposits too much Y
    // lp_x = 10 * 200 / 100 = 20
    // lp_y = 60 * 200 / 400 = 30
    // lp   = min(20,30) = 20
     */
    #[test]
    fn test_deposit_lp_wrong_ratio_takes_minimum() {
        let result = calculate_deposit_lp(100, 400, 200, 10, 60).unwrap();
        assert_eq!(result, 20);
    }

    // vault_x is zero
    #[test]
    fn test_deposit_lp_zero_vault_fails() {
        assert!(calculate_deposit_lp(0, 400, 200, 10, 40).is_err());
    }

    // pool: 1000 X, 2000 Y (price = 2 Y per X)
    // user swaps 100 X to gets Y
    // amount_in_after_fee = 100 * 9970 / 10000 = 99
    // dy = 2000 * 99 / (1000 + 99) = 180
    #[test]
    fn test_swap_x_to_y_direction() {
        let (out, fee) = swap_tokens(1000, 2000, 100, 30, true).unwrap();
        assert_eq!(fee, 1);
        assert_eq!(out, 180);
    }

    #[test]
    fn test_swap_y_to_x_direction() {
        let (out, fee) = swap_tokens(1000, 2000, 200, 30, false).unwrap();
        assert_eq!(fee, 1);
        assert_eq!(out, 90);
    }

    #[test]
    fn test_withdraw() {
        let (x_out, y_out) = calculate_withdraw(1000, 2000, 1414, 707).unwrap();
        assert_eq!(x_out, 500);
        assert_eq!(y_out, 1000);
    }
}
