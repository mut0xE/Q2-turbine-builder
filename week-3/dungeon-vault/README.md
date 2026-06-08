# Dungeon Vault

An Anchor Vault program on Solana where players deposit SOL into a shared vault and the winner withdraws the entire pool. Built with [MagicBlock Ephemeral Rollups](https://docs.magicblock.gg/pages/ephemeral-rollups-ers/how-to-guide/quickstart) for gameplay and Ephemeral VRF for on-chain randomness.

## How It Works

- A creator initializes a dungeon vault with an entry fee and max player count (2-4).
- Players join by depositing SOL into the vault PDA.
- Each round, players pick a door (1-3). A VRF oracle picks a trap door. Players who chose the trap are eliminated.
- Last player standing claims the full vault. On a draw, the creator reclaims it.

\`\`\`mermaid
flowchart LR
    A[Initialize Vault] --> B[Players Join & Deposit SOL]
    B --> C[Submit Choices]
    C --> D[VRF Selects Trap]
    D --> E{Eliminated?}
    E -- Yes --> F[Player Out]
    E -- No --> C
    E -- Last Standing --> G[claim_winner: Player Claims Vault]
    E -- All Eliminated --> H[claim_draw: Creator Reclaims Vault]
\`\`\`

## Program Instructions

- `initialize_dungeon` — Create a new vault with entry fee and player limit
- `join_dungeon` — Deposit SOL into the vault and join the game
- `submit_choice` — Pick a door (1, 2, or 3)
- `request_randomness` — Request VRF randomness for trap selection
- `resolve_round` — Eliminate players who picked the trap door; only callable by the dungeon authority
- `claim_winner` — Last surviving player withdraws the full vault
- `claim_draw` — Creator reclaims the vault when all players are eliminated in the same round
- `delegate_account` / `undelegate` — Delegate accounts to Ephemeral Rollups for gameplay

## Security

- `resolve_round` enforces `has_one = authority` so only the dungeon creator can resolve rounds. Each player state PDA is re-derived from its internal player pubkey and cross-checked against the passed account key, preventing both fake account injection and omission of active players.
- `claim_winner` and `claim_draw` are separate instructions so the creator draw path never requires a `player_state` account. Winner identity is enforced at the account constraint level via PDA seeds, not runtime pubkey checks.

## Prerequisites

- Rust (1.89.0)
- Solana CLI with a devnet wallet
- Anchor CLI v0.32.x
- Node.js and Yarn

## Build and Test

\`\`\`bash
yarn install
anchor build && anchor deploy
anchor test --skip-deploy
\`\`\`

> Tested on Solana devnet. Requires a funded devnet wallet

## Tests

The test suite covers all instructions:

- Initialize dungeon with valid and invalid parameters
- Players joining and SOL depositing into the vault
- Rejecting joins when the game is full
- Account delegation to Ephemeral Rollups
- Full gameplay loop (choices, VRF, round resolution, elimination)
- Undelegation back to mainnet
- Winner claiming the vault via `claim_winner`
- Creator reclaiming the vault via `claim_draw` on a draw outcome

## Program ID

\`\`\`
CuXrhPFnmbt2Ktnpk5RXCR56oLnu9165hyt1zxvCGn7W
\`\`\`
