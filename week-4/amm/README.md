# AMM — Constant Product Automated Market Maker

A Solana program built with Anchor that implements a constant-product (`x * y = k`) automated market maker. Users can create token-pair pools, deposit liquidity, swap tokens, and withdraw their share — all on-chain with PDA-secured vaults.

**Program ID:** `9skP2HrosgroRxykvVwF1K4w4FJPTxeuJSpHrgcvRrDK`

---

## How It Works

Two tokens go into a pool. Traders swap between them using the `x * y = k` curve and pay a small fee. That fee stays in the pool and goes to liquidity providers who deposited the tokens.

See the full architecture: [docs/architecture.md](docs/architecture.md)

---

## Instructions

### Initialize

Creates a new pool with its config, vaults, and LP mint. The caller becomes the authority.

- `index` (u64) — unique pool identifier, used as PDA seed
- `fee` (Option\<u16\>) — basis points, defaults to 30 (0.3%), max 100 (1.0%)

Full flow with sequence diagram: [docs/initialize.md](docs/initialize.md)

### Deposit

Adds liquidity. Transfers tokens X and Y into the vaults, mints LP tokens to the provider.

- First deposit: `LP = sqrt(amount_x * amount_y)`
- Subsequent: `LP = min(dx * supply / X, dy * supply / Y)`
- Reverts if pool is locked or if minted LP is below `min_lp`

Full flow with LP calculation diagram: [docs/deposit.md](docs/deposit.md)

### Swap

Exchanges one token for the other. Fee is deducted from input before the `x * y = k` calculation.

```
fee       = amount_in * fee_rate / 10000
dx'       = amount_in - fee
amount_out = vault_out * dx' / (vault_in + dx')
```

The fee stays in the vault, growing reserves and increasing LP token value over time.

Full flow with fee diagram: [docs/swap.md](docs/swap.md)

### Withdraw

Burns LP tokens and returns a proportional share of both reserves.

```
x_out = vault_x * lp_amount / lp_supply
y_out = vault_y * lp_amount / lp_supply
```

Withdrawals always work, even when the pool is locked.

Full flow: [docs/withdraw.md](docs/withdraw.md)

### Update Config

Authority-only. Can modify fee rate, lock/unlock the pool, transfer authority, or permanently renounce it. Once renounced, the config becomes immutable.

Full flow: [docs/update-config.md](docs/update-config.md)

---

## Quick Start

```bash
# install dependencies
yarn install

# build the program
anchor build

# run tests on localnet (surfpool / solana-test-validator)
anchor test

# run tests on devnet
# 1. change Anchor.toml [provider] cluster = "devnet"
# 2. fund your wallet: solana airdrop 2 --url devnet
anchor test --skip-local-validator
```

---

## Deployment

```bash
# devnet
anchor deploy --provider.cluster devnet

# mainnet
anchor deploy --provider.cluster mainnet
```

After deploying to a new cluster, update the program ID in:
1. `programs/amm/src/lib.rs` — `declare_id!(...)`
2. `Anchor.toml` — `[programs.<cluster>]`
3. `tests/pda.ts` — `PROGRAM_ID`
