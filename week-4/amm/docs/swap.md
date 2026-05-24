# Swap

Exchanges one token for the other using the constant-product (`x * y = k`) invariant. A fee is deducted from the input before the swap calculation.

```mermaid
sequenceDiagram
    participant User
    participant Program as AMM Program
    participant VX as Vault X
    participant VY as Vault Y

    User->>Program: swap(amount_in, min_out, x_to_y)
    Note over Program: Reject if pool is locked
    Note over Program: Reject if amount_in is zero
    Note over Program: fee = amount_in * fee_rate / 10000
    Note over Program: dx' = amount_in - fee
    Note over Program: amount_out = Y * dx' / (X + dx')
    Note over Program: Reject if amount_out < min_out

    alt x_to_y = true (sell X, buy Y)
        User->>VX: transfer_checked(amount_in)
        VY->>User: transfer_checked(amount_out) [pool PDA signs]
    else x_to_y = false (sell Y, buy X)
        User->>VY: transfer_checked(amount_in)
        VX->>User: transfer_checked(amount_out) [pool PDA signs]
    end

    Program-->>User: emit SwapEvent
```

## Parameters

| Name | Type | Description |
|------|------|-------------|
| `amount_in` | `u64` | Tokens the user sends in |
| `min_out` | `u64` | Minimum tokens to receive (slippage protection) |
| `x_to_y` | `bool` | `true` = send X, receive Y. `false` = send Y, receive X |

## Fee Handling

```mermaid
graph LR
    Input["amount_in<br/>(e.g. 100 tokens)"] --> Fee["Fee portion<br/>(e.g. 0.3 tokens)<br/>stays in vault"]
    Input --> Net["Net input<br/>(e.g. 99.7 tokens)<br/>used in x*y=k calc"]
    Net --> Output["amount_out<br/>sent to user"]

    style Fee fill:#f96,stroke:#333
    style Net fill:#6f9,stroke:#333
```

The fee tokens stay inside the vault. This grows the reserves relative to LP supply, which means each LP token is backed by more underlying tokens over time. That growth is how liquidity providers earn yield.

## Constant Product Formula

```
amount_out = vault_out * dx' / (vault_in + dx')
```

Where `dx' = amount_in - fee`. The product `vault_in * vault_out` is preserved (increases slightly due to the fee), maintaining the `k` invariant.
