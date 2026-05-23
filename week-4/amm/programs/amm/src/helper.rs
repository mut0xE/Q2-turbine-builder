use anchor_lang::prelude::*;

use crate::errors::AmmError;

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
}
