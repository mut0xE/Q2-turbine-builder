# Update Config

Authority-only instruction. Modifies the pool configuration: fee rate, lock state, authority transfer, or permanent renouncement.

```mermaid
sequenceDiagram
    participant Auth as Authority
    participant Program as AMM Program
    participant Config as AmmConfig
    participant Pool as Pool

    Auth->>Program: update_config(new_fee?, locked?, new_authority?, renounce)

    Note over Program: Reject if authority is None (renounced)
    Note over Program: Reject if signer != config.authority

    opt new_fee provided
        Note over Program: Reject if fee >= 100 bps
        Program->>Config: config.fee_rate = new_fee
    end

    opt locked provided
        Program->>Pool: pool.locked = locked
    end

    alt renounce = true
        Program->>Config: config.authority = None
        Note over Config: Permanent. No further updates possible.
    else new_authority provided
        Program->>Config: config.authority = new_authority
    end

    Program-->>Auth: success
```

## Parameters

| Name | Type | Description |
|------|------|-------------|
| `new_fee` | `Option<u16>` | New fee in basis points (must be < 100) |
| `locked` | `Option<bool>` | `true` pauses deposits and swaps, `false` resumes |
| `new_authority` | `Option<Pubkey>` | Transfer authority to a different address |
| `renounce` | `bool` | If `true`, sets authority to `None` permanently |

## Lock Behavior

When `pool.locked = true`:
- `deposit` reverts with `PoolLocked`
- `swap` reverts with `PoolLocked`
- `withdraw` still works (users can always exit)

## Renounce

Setting `renounce = true` writes `None` to `config.authority`. After this, any call to `update_config` will fail with `AuthorityRenounced`. There is no way to undo this. The fee rate and lock state become frozen at their current values.
