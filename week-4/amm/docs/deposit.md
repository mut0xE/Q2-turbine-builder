# Deposit

Adds liquidity to an existing pool. Transfers tokens X and Y from the user into the pool vaults and mints LP tokens back to the user.

```mermaid
sequenceDiagram
    participant LP as LP Provider
    participant Program as AMM Program
    participant VX as Vault X
    participant VY as Vault Y
    participant LPM as LP Mint

    LP->>Program: deposit(amount_x, amount_y, min_lp)
    Note over Program: Reject if pool is locked
    Note over Program: Reject if amounts are zero
    Note over Program: Calculate LP tokens to mint

    Program->>VX: transfer_checked(amount_x) from user ATA X
    Program->>VY: transfer_checked(amount_y) from user ATA Y
    Program->>LPM: mint_to(lp_amount) to user LP ATA
    Program-->>LP: emit DepositEvent
```

## Parameters

| Name | Type | Description |
|------|------|-------------|
| `amount_x` | `u64` | Token X to deposit |
| `amount_y` | `u64` | Token Y to deposit |
| `min_lp` | `u64` | Minimum LP tokens to receive (slippage protection) |

## LP Calculation

```mermaid
graph TD
    A{Is pool empty?}
    A -->|Yes, first deposit| B["LP = sqrt(amount_x * amount_y)"]
    A -->|No, pool has reserves| C["lp_x = dx * supply / X"]
    C --> D["lp_y = dy * supply / Y"]
    D --> E["LP = min(lp_x, lp_y)"]
```

- **First deposit** uses the geometric mean — there is no existing ratio to follow.
- **Subsequent deposits** calculate LP contribution from each side independently and take the minimum. This prevents a depositor from skewing the ratio to extract value.
- If the calculated LP is below `min_lp`, the transaction reverts with `SlippageExceeded`.
