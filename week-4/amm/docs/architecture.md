# Architecture Overview

High-level view of how users interact with the AMM program and the on-chain accounts it manages.

```mermaid
graph TB
    subgraph Users
        LP[Liquidity Provider]
        Trader[Trader]
        Admin[Authority]
    end

    subgraph Program["AMM Program"]
        Init[Initialize]
        Dep[Deposit]
        Sw[Swap]
        Wd[Withdraw]
        Upd[Update Config]
    end

    subgraph On-Chain State
        Config[AmmConfig]
        Pool[Pool]
        VX[Vault X<br/>PDA Token Account]
        VY[Vault Y<br/>PDA Token Account]
        LPM[LP Mint<br/>PDA Mint]
    end

    LP -->|create pool| Init
    LP -->|add liquidity| Dep
    LP -->|remove liquidity| Wd
    Trader -->|exchange tokens| Sw
    Admin -->|fee / lock / authority| Upd

    Init -->|creates| Config
    Init -->|creates| Pool
    Init -->|creates| VX
    Init -->|creates| VY
    Init -->|creates| LPM

    Dep -->|transfers tokens into| VX
    Dep -->|transfers tokens into| VY
    Dep -->|mints| LPM

    Sw -->|tokens in| VX
    Sw -->|tokens out| VY

    Wd -->|burns LP from| LPM
    Wd -->|sends tokens from| VX
    Wd -->|sends tokens from| VY

    Upd -->|modifies| Config
    Upd -->|locks/unlocks| Pool
```

Three user roles exist:

- **Liquidity Provider** — creates pools, deposits token pairs, withdraws by burning LP tokens.
- **Trader** — swaps one token for the other through the constant-product curve.
- **Authority** — the address that initialized the pool. Can update fees, lock/unlock the pool, transfer authority, or permanently renounce it.

Every account the program creates (config, pool, vaults, LP mint) is a PDA — no keypairs are stored, all addresses are deterministic from seeds.
