# Metaplex Core

This document explains the Metaplex Core concepts and plugins used in this NFT staking program.

Official documentation: [https://www.metaplex.com/docs/smart-contracts/core](https://www.metaplex.com/docs/smart-contracts/core)

## What is Metaplex Core?

Metaplex Core (MPL Core) is Metaplex's latest NFT standard on Solana. Unlike the older Token Metadata standard which requires multiple accounts per NFT (mint, metadata, token account, master edition), MPL Core stores everything in a single account. This reduces cost, simplifies the account model, and makes NFTs cheaper to create and manage.

Key differences from the legacy standard:

- Single account per asset (no separate mint, metadata, or edition accounts)
- Built-in plugin system for extensible behavior
- Collection-level enforcement of rules
- No dependency on SPL Token for the NFT itself

## Collections

A **Collection** in MPL Core is a standalone on-chain account that groups related assets. Collections are created via `CreateCollectionV2` and have their own metadata (name, uri) and plugins.

In this program, the collection:

- Is created as a regular keypair account (not a PDA)
- Has an `Attributes` plugin with a `staked_count` field
- Uses a PDA (`update_authority`) as its update authority so the program can modify collection attributes via CPI

```
Collection Account
  - name: "My Collection"
  - uri: "https://..."
  - update_authority: PDA (program-controlled)
  - plugins:
      Attributes: { staked_count: "0" }
```

## Assets

An **Asset** is an individual NFT in MPL Core. Each asset is a single account containing ownership info, metadata, and plugins. Assets belong to a collection through their `update_authority` field, which is set to `UpdateAuthority::Collection(collection_address)`.

In this program, each asset has:

- An owner (the wallet that minted it)
- A link to its collection via `update_authority`
- An `Attributes` plugin with User, Timestamp, and Staked fields

```
Asset Account
  - owner: <user wallet>
  - update_authority: Collection(<collection address>)
  - name: "NFT #1"
  - uri: "https://..."
  - plugins:
      Attributes: {
        User: "<pubkey>",
        Timestamp: "<unix timestamp>",
        Staked: "false"
      }
```

## Update Authority

The **Update Authority** controls who can modify an asset's or collection's metadata and plugins. In MPL Core, update authority can be:

- A wallet address
- A collection (for assets within that collection)
- A PDA (for program-controlled updates)

In this program, the update authority is a PDA derived from `["update_authority", collection_pubkey]`. This allows the staking program to:

- Update asset attributes (flip `Staked` between "true" and "false")
- Update collection attributes (increment/decrement `staked_count`)

The PDA signs these operations via `invoke_signed`, so no external wallet can modify these values.

## Plugins Used

MPL Core uses a plugin system to add behavior to assets and collections. Plugins are attached to accounts and define rules for lifecycle events (create, transfer, burn, update, etc.).

### Attributes Plugin

The `Attributes` plugin stores arbitrary key-value pairs on an asset or collection. It is used here for on-chain metadata:

**On Collections:**
```
Attributes {
  staked_count: "0"    // tracks how many assets are currently staked
}
```

**On Assets:**
```
Attributes {
  User: "<owner pubkey>",       // who owns this NFT
  Timestamp: "<unix time>",     // when it was minted/last staked
  Staked: "true" | "false"      // current staking status
}
```

The update authority PDA has permission to modify these attributes. This is set during collection and asset creation when the update authority is assigned.

### FreezeDelegate Plugin

The `FreezeDelegate` plugin prevents an asset from being transferred, burned, or sold while active and frozen. It is an **Owner Managed** plugin, meaning:

- The **owner** must authorize adding or removing the plugin
- The designated **authority** (delegate) can toggle the `frozen` state
- Only the **owner** can remove the plugin (after unfreezing)

In this program's staking flow:

1. **Stake**: The owner signs to add FreezeDelegate with `frozen: true`. The `stake_info` PDA is set as the freeze authority via `init_authority`.

```
FreezeDelegate {
  frozen: true,
  authority: Address(stake_info_pda)
}
```

2. **Unstake**: Two steps are required:
   - The `stake_info` PDA (freeze authority) signs to set `frozen: false`
   - The owner signs to remove the FreezeDelegate plugin entirely

This design ensures that:
- While staked, only the program can unfreeze the asset (via the stake_info PDA)
- The owner cannot transfer or sell the asset while it is frozen
- After unstaking, the asset is fully unlocked with no remaining plugin

## Plugin Authority Types

MPL Core categorizes plugins by who manages them:

| Category | Who can add/remove | Who can update | Used in this program |
|----------|-------------------|----------------|---------------------|
| Owner Managed | Owner | Designated authority | FreezeDelegate |
| Authority Managed | Update authority | Update authority | Attributes |

- **FreezeDelegate** is Owner Managed: the owner must sign to add it during staking and to remove it during unstaking. The designated authority (stake_info PDA) can toggle the frozen state.
- **Attributes** is Authority Managed: the update authority PDA can freely modify attribute values via CPI.

## Lifecycle Events

MPL Core plugins participate in lifecycle events. Each plugin can approve, reject, or abstain from an event:

| Event | FreezeDelegate behavior |
|-------|------------------------|
| Transfer | Rejects if `frozen: true` |
| Burn | Rejects if `frozen: true` |
| Update | Approves (attributes can still be updated) |
| Remove Plugin | Rejects if `frozen: true` (must unfreeze first) |

This is why the unstake instruction must first set `frozen: false` before calling `RemovePlugin`. Attempting to remove a frozen FreezeDelegate will be rejected by MPL Core.

## References

- [Metaplex Core Documentation](https://www.metaplex.com/docs/smart-contracts/core)
- [MPL Core Rust Crate](https://crates.io/crates/mpl-core)
- [MPL Core JS SDK](https://www.npmjs.com/package/@metaplex-foundation/mpl-core)
