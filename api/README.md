# oil-api

API for interacting with the OIL protocol on Solana. OIL is a crypto mining protocol that allows users to deploy SOL to claim space on a board, mine OIL tokens, and stake for yield.

[![crates.io](https://img.shields.io/crates/v/oil-api.svg)](https://crates.io/crates/oil-api)
[![docs.rs](https://docs.rs/oil-api/badge.svg)](https://docs.rs/oil-api)

## Features

- **Instruction builders** for all OIL protocol instructions
- **State types** and account deserialization for all program accounts
- **PDA derivation utilities** for deriving program-derived addresses
- **SDK functions** for common operations like deploying, claiming, and staking
- **Type-safe** instruction and state handling using `steel` and `bytemuck`

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
oil-api = "0.3.3"
```

## Example

```rust
use oil_api::prelude::*;
use solana_program::pubkey::Pubkey;

// Derive a Miner PDA
let (miner_pda, _bump) = miner_pda(&signer.pubkey());

// Build a Deploy instruction
let deploy_ix = deploy(
    &signer.pubkey(),
    amount,           // Amount in lamports
    square_id,        // Square ID (0-24)
    referrer,         // Optional referrer address
);

// Build a ClaimOIL instruction
let claim_oil_ix = claim_oil(&signer.pubkey());

// Build a Deposit instruction for staking
let deposit_ix = deposit(
    &signer.pubkey(),
    amount,           // Amount in OIL (grams)
);
```

## Modules

### Core Modules

- **`consts`** - Program constants including addresses, time constants, and token decimals
- **`error`** - Custom program error types
- **`event`** - Program event types
- **`instruction`** - Instruction types and builders
- **`sdk`** - High-level SDK functions for common operations
- **`state`** - Account state types and PDA derivation functions

### State Accounts

- **`Auction`** - Auction state
- **`Automation`** - Automation configuration
- **`Bid`** - Auction bid account
- **`Board`** - Current round state
- **`Config`** - Global program configuration
- **`Miner`** - User mining state
- **`Pool`** - Mining pool state
- **`AuctionPool`** - Auction pool state
- **`Referral`** - Referral tracking
- **`Rig`** - Rig configuration
- **`Round`** - Round-specific state
- **`Stake`** - Staking account
- **`Treasury`** - Treasury account
- **`Well`** - Well state

## Instructions

### Mining Instructions
- `deploy` - Deploy SOL to claim space on the board
- `claim_oil` - Claim OIL mining rewards
- `claim_sol` - Claim SOL mining rewards
- `checkpoint` - Checkpoint rewards from a prior round
- `automate` - Configure automation
- `reload_sol` - Reload SOL mining rewards into automation
- `reset` - Reset the board for a new round

### Auction Instructions
- `place_bid` - Set a bid in the auction (automatically syncs accumulated OIL and halvings)
- `claim_auction_oil` - Claim OIL from auction
- `claim_auction_sol` - Claim SOL from auction

### Staking Instructions
- `deposit` - Deposit OIL into a stake account
- `withdraw` - Withdraw OIL from a stake account
- `claim_yield` - Claim staking yield

### Referral Instructions
- `create_referral` - Create a referral account
- `claim_referral` - Claim referral rewards

## Program ID

The OIL program ID is: `rigwXYKkE8rXiiyu6eFs3ZuDNH2eYHb1y87tYqwDJhk`

## Documentation

Full API documentation is available at:
- **docs.rs**: https://docs.rs/oil-api
- **Repository**: https://github.com/oil-protocol/oil

## Dependencies

This crate depends on:
- `entropy-rng-api` - For entropy-based randomness
- `oil-mint-api` - For OIL token minting operations
- `solana-program` - Solana program SDK
- `steel` - Solana instruction building framework
- `spl-token` - SPL token program
- `spl-token-2022` - SPL token 2022 program

## License

Licensed under Apache-2.0

## Related

- [OIL Program](https://github.com/oil-protocol/oil) - The on-chain Solana program
- [OIL Protocol](https://oil.supply) - Protocol website
