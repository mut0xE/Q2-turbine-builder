# Withdraw

Removes liquidity from the pool. Burns the user's LP tokens and returns a proportional share of both token X and token Y from the vaults.

```mermaid
sequenceDiagram
    participant User
    participant Program as AMM Program
    participant LPM as LP Mint
    participant VX as Vault X
    participant VY as Vault Y

    User->>Program: withdraw(lp_amount, min_x, min_y)
    Note over Program: Reject if lp_amount is zero
    Note over Program: x_out = X * lp_amount / lp_supply
    Note over Program: y_out = Y * lp_amount / lp_supply
    Note over Program: Reject if x_out < min_x or y_out < min_y

    Program->>LPM: burn(lp_amount) from user LP ATA
    VX->>User: transfer_checked(x_out) [pool PDA signs]
    VY->>User: transfer_checked(y_out) [pool PDA signs]
    Program-->>User: emit WithdrawEvent
```

## Parameters

| Name | Type | Description |
|------|------|-------------|
| `lp_amount` | `u64` | LP tokens to burn |
| `min_x` | `u64` | Minimum token X to receive (slippage protection) |
| `min_y` | `u64` | Minimum token Y to receive (slippage protection) |

## Withdrawal Formula

```
x_out = vault_x * lp_amount / lp_supply
y_out = vault_y * lp_amount / lp_supply
```

The withdrawal is strictly proportional. Burning 50% of LP supply returns 50% of each reserve. Because swap fees continuously grow the vault balances, the tokens returned will be more than what was originally deposited — that difference is the LP yield.

LP tokens are burned **before** the vault transfers. This ordering ensures the supply is already reduced, preventing reentrancy-style issues.
