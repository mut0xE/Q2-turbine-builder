# NFT Staking Program

A Solana on-chain program built with Anchor that allows users to stake MPL Core NFTs and earn SPL token rewards over time.

## Overview

This program implements a complete NFT staking system using Metaplex Core assets. Users can:

- Stake their NFTs to earn reward tokens
- Claim accumulated rewards based on time staked
- Unstake after a configurable freeze period

The program uses MPL Core's FreezeDelegate plugin to lock assets during staking, preventing transfers or sales while staked. Rewards are minted from a program-controlled SPL token mint based on a configurable basis points rate per day.

## Architecture

### Accounts (PDAs)

| Account | Seeds | Purpose |
|---------|-------|---------|
| Config | `["config"]` | Stores reward rate, freeze period, and PDA bumps |
| Reward Mint | `["rewards", config]` | SPL token mint controlled by the program |
| Update Authority | `["update_authority", collection]` | Signs for collection/asset attribute updates |
| Stake Info | `["stake", asset, owner]` | Tracks stake timestamp and last claim time per asset |

### Instructions

| Instruction | Description |
|-------------|-------------|
| `initialize` | Creates config PDA and reward token mint. Sets reward rate (basis points/day) and freeze period (days). |
| `create_collection` | Creates an MPL Core collection with a `staked_count` attribute. |
| `mint_asset` | Mints an NFT into a collection with User, Timestamp, and Staked attributes. |
| `stake` | Freezes the asset via FreezeDelegate, marks it as staked, creates StakeInfo PDA, increments collection staked_count. |
| `claim_rewards` | Calculates rewards based on elapsed days since last claim, mints reward tokens to the owner. |
| `unstake` | Checks freeze period, unfreezes the asset, removes FreezeDelegate, updates attributes, closes StakeInfo PDA. |

### Reward Calculation

```
elapsed_days = (now - last_claimed) / 86400
reward = elapsed_days * rewards_bps * 10^6 / 10_000
```

- `rewards_bps` is in basis points (100 = 1% per day)
- Rewards are minted with 6 decimal places
- Minimum 1 full day must pass before claiming


## Prerequisites

- Rust and Cargo
- Solana CLI
- Anchor CLI (0.31.x)
- Node.js and Yarn
- Surfpool 

## Build

```bash
anchor build
```

## Test

```bash
# Start surfpool (in a separate terminal)
surfpool start

# Run tests surfpool
anchor test --skip-deploy
```

## Configuration

The `initialize` instruction accepts:

- `rewards_bps` (u16) -- Reward rate in basis points per day. 100 = 1%/day.
- `freeze_period` (u16) -- Minimum days an NFT must stay staked before unstaking.

## Documentation

- [docs/staking-flow.md](docs/staking-flow.md) -- Staking lifecycle diagram
- [docs/metaplex-core.md](docs/metaplex-core.md) -- MPL Core concepts and plugins
