# Initialize

Sets up everything the AMM needs before any trading can happen. A single call to `initialize` creates the config, pool, LP mint, and both token vaults — all derived as PDAs so their addresses are deterministic.

```mermaid
sequenceDiagram
    participant Payer
    participant Program as AMM Program
    participant Config as AmmConfig PDA
    participant Pool as Pool PDA
    participant VX as Vault X PDA
    participant VY as Vault Y PDA
    participant LPM as LP Mint PDA

    Payer->>Program: initialize(index, fee?)
    Program->>Config: init (fee, authority=payer, index, bump)
    Program->>Pool: init (config, mint_x, mint_y, lp_mint, bumps, locked=false)
    Program->>LPM: init mint (decimals=6, authority=pool)
    Program->>VX: init token account (mint=X, authority=pool)
    Program->>VY: init token account (mint=Y, authority=pool)
    Program-->>Payer: success
```

The instruction takes an `index` (u64) to uniquely identify this pool's config and an optional `fee` in basis points (defaults to 30 / 0.3%, max 100 / 1.0%). The payer who signs the transaction is set as the config authority and pays rent for all five accounts. The pool starts unlocked and with empty vaults — it's ready for the first `deposit`.
