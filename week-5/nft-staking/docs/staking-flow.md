# NFT Staking Flow

This document describes the lifecycle of staking an MPL Core NFT, from initialization through claiming rewards and unstaking.

## Full Lifecycle Diagram

```mermaid
sequenceDiagram
    participant Admin
    participant Program
    participant User
    participant MPL Core
    participant Token Program

    Note over Admin, Program: 1. Initialize
    Admin->>Program: initialize(rewards_bps, freeze_period)
    Program->>Program: Create Config PDA
    Program->>Token Program: Create Reward Mint PDA
    Note right of Program: Config stores reward rate,<br/>freeze period, and bumps

    Note over Admin, MPL Core: 2. Create Collection
    Admin->>Program: create_collection(name, uri)
    Program->>MPL Core: CreateCollectionV2 CPI
    Note right of MPL Core: Collection created with<br/>Attributes plugin:<br/>staked_count = 0

    Note over User, MPL Core: 3. Mint Asset
    User->>Program: mint_asset(name, uri)
    Program->>MPL Core: CreateV2 CPI
    Note right of MPL Core: Asset created with attributes:<br/>User, Timestamp, Staked=false

    Note over User, MPL Core: 4. Stake
    User->>Program: stake()
    Program->>MPL Core: AddPluginV1 (FreezeDelegate, frozen=true)
    Program->>MPL Core: UpdatePluginV1 (Staked=true)
    Program->>Program: Create StakeInfo PDA
    Program->>MPL Core: UpdateCollectionPluginV1 (staked_count++)
    Note right of Program: Asset is now frozen,<br/>cannot be transferred

    Note over User, Token Program: 5. Claim Rewards
    User->>Program: claim_rewards()
    Program->>Program: Calculate elapsed days since last claim
    Program->>Token Program: mint_to (reward tokens to user ATA)
    Program->>Program: Update last_claimed timestamp
    Note right of Program: Rewards = days * bps * 10^6 / 10000

    Note over User, MPL Core: 6. Unstake
    User->>Program: unstake()
    Program->>Program: Check freeze_period has passed
    Program->>MPL Core: UpdatePluginV1 (FreezeDelegate, frozen=false)
    Program->>MPL Core: RemovePluginV1 (FreezeDelegate)
    Program->>MPL Core: UpdatePluginV1 (Staked=false)
    Program->>MPL Core: UpdateCollectionPluginV1 (staked_count--)
    Program->>Program: Close StakeInfo PDA (rent returned to owner)
    Note right of Program: Asset is now unfrozen,<br/>owner can transfer freely
```

## State Transitions

```mermaid
stateDiagram-v2
    [*] --> Minted: mint_asset()
    Minted --> Staked: stake()
    Staked --> Staked: claim_rewards()
    Staked --> Minted: unstake()
    Minted --> Staked: stake() (restake)

    state Minted {
        [*] --> Unfrozen
        note right of Unfrozen
            Staked = false
            No FreezeDelegate
            Transferable
        end note
    }

    state Staked {
        [*] --> Frozen
        note right of Frozen
            Staked = true
            FreezeDelegate active
            Not transferable
            Earning rewards
        end note
    }
```

## PDA Relationships

```mermaid
graph TD
    A[Config PDA] -->|seeds: config| B[Reward Mint PDA]
    B -->|seeds: rewards + config| B
    C[Collection] -->|seeds: update_authority + collection| D[Update Authority PDA]
    E[Asset] -->|seeds: stake + asset + owner| F[StakeInfo PDA]

    A -.->|mint authority| B
    D -.->|signs attribute updates| C
    D -.->|signs attribute updates| E
    F -.->|freeze authority| E

    style A fill:#f0f0f0,stroke:#333
    style B fill:#f0f0f0,stroke:#333
    style D fill:#f0f0f0,stroke:#333
    style F fill:#f0f0f0,stroke:#333
```

## Freeze Period

The freeze period is configured during initialization and measured in days. When a user stakes an NFT:

1. `staked_at` timestamp is recorded in the StakeInfo PDA
2. When unstaking, the program checks: `(now - staked_at) >= freeze_period * 86400`
3. If the freeze period has not passed, the unstake is rejected with `FreezePeriodNotPassed`

This prevents users from gaming the system by rapidly staking and unstaking.

## Reward Calculation

Rewards are calculated on each `claim_rewards` call:

1. `elapsed = now - last_claimed` (seconds)
2. `elapsed_days = elapsed / 86400` (integer division, partial days are not counted)
3. `reward_amount = elapsed_days * rewards_bps * 1_000_000 / 10_000`

The `last_claimed` timestamp is updated to `now` after each claim. The `staked_at` timestamp is never modified, so the freeze period always counts from the original stake time.
