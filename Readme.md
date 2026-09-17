# AMM Program

A constant-product (`x*y=k`) Automated Market Maker built with Anchor on Solana. Supports pool initialization, liquidity deposits/withdrawals, token swaps, and protocol fee collection to a treasury.

## Overview

The program lets anyone create a liquidity pool for a pair of SPL token mints (`mint_x`, `mint_y`), deposit both tokens to earn LP tokens representing their share, swap one token for the other along a constant-product curve, and withdraw their share back out. A configurable protocol fee is skimmed from swaps and routed to a treasury account.

## Accounts

### `AmmConfig` (PDA — seeds: `[b"amm_config", seed.to_le_bytes()]`)

The pool's state/config account. One per `(seed)` — the same `mint_x`/`mint_y` pair can have multiple pools under different seeds.

| Field | Type | Description |
|---|---|---|
| `seed` | `u64` | Arbitrary value used to derive this pool's PDA, allowing multiple pools per mint pair |
| `authority` | `Option<Pubkey>` | Pool creator/admin |
| `mint_x` / `mint_y` | `Pubkey` | The two token mints this pool trades |
| `lp_mint` | `Pubkey` | Mint for this pool's LP tokens |
| `treasury` | `Pubkey` | Wallet that receives protocol fees |
| `locked` | `bool` | Emergency pause switch |
| `fee` | `u16` | Swap fee in basis points (applied by the curve, paid to LPs) |
| `protocol_fee` | `u16` | Additional basis points skimmed off swap input to the treasury |
| `bump` | `u8` | PDA bump |

### Vaults & LP mint
- `vault_x`, `vault_y` — ATAs owned by the `config` PDA; hold the pool's actual reserves.
- `lp_mint` — PDA (seeds: `[b"lp", config.key()]`), mint authority = `config`. Minted to depositors, burned on withdrawal.

## Instructions

### `initialize(seed: u64, fee: u16, protocol_fee: u16)`
Creates the pool: `AmmConfig`, `lp_mint`, `vault_x`, `vault_y`. No tokens move.

### `deposit(amount: u64, max_x: u64, max_y: u64)`
Adds liquidity. On an empty pool, the caller sets the initial `x`/`y` amounts directly (`max_x`, `max_y`). On a non-empty pool, required `x`/`y` amounts are derived proportionally from `amount` (LP tokens requested) via the curve, and capped by `max_x`/`max_y` as slippage protection. Mints `amount` LP tokens to the depositor.

### `swap(is_x: bool, amount: u64, min: u64)`
Swaps `amount` of token X (if `is_x`) or Y for the other token. Flow:
1. `protocol_fee` bps of `amount` is skimmed off the top and transferred straight to the treasury's ATA.
2. The remainder runs through the constant-product curve (which applies `fee` internally) to compute the output.
3. Output must be `>= min`, or the instruction fails (slippage protection).

### `withdraw(amount: u64, min_x: u64, min_y: u64)`
Burns `amount` LP tokens and returns a proportional share of both vault reserves, each required to be `>= min_x` / `min_y`.

## Errors

| Error | Meaning |
|---|---|
| `InvalidAmount` | Amount was zero, or fee ≥ 10,000 bps |
| `PoolLocked` | Pool's `locked` flag is set |
| `SlippageExceeded` | Output/required amount violated the caller's min/max bound |
| `InsufficientFunds` | User doesn't hold enough LP tokens for the requested withdrawal |

## Project Structure

```
programs/amm-program/src/
├── lib.rs              # program entrypoints
├── state.rs             # AmmConfig account
├── error.rs              # AmmError enum
├── constants.rs
└── instructions/
    ├── initialize.rs
    ├── deposit.rs
    ├── swap.rs
    └── withdraw.rs
tests/
└── amm_tests.rs          # LiteSVM integration test (full init→deposit→swap→withdraw flow)
```

