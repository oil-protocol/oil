# OIL

OIL is a crypto mining protocol.


## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Event`](api/src/error.rs) – Custom program events.
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments.

## Instructions

#### Mining
- [`Automate`](program/src/automate.rs) - Configures a new automation.
- [`AutomateWithSession`](program/src/automate_with_session.rs) - Configures a new automation (Fogo session).
- [`Checkpoint`](program/src/checkpoint.rs) - Checkpoints rewards from a prior round.
- [`CheckpointWithSession`](program/src/checkpoint_with_session.rs) - Checkpoints rewards from a prior round (Fogo session).
- [`ClaimOIL`](program/src/claim_oil.rs) - Claims OIL mining rewards.
- [`ClaimOILWithSession`](program/src/claim_oil_with_session.rs) - Claims OIL mining rewards (Fogo session).
- [`ClaimSOL`](program/src/claim_sol.rs) - Claims SOL mining rewards.
- [`ClaimSOLWithSession`](program/src/claim_sol_with_session.rs) - Claims SOL mining rewards (Fogo session).
- [`Deploy`](program/src/deploy.rs) - Deploys SOL to claim space on the board.
- [`DeployWithSession`](program/src/deploy_with_session.rs) - Deploys SOL to claim space on the board (Fogo session).
- [`Initialize`](program/src/initialize.rs) - Initializes program variables.
- [`Log`](program/src/log.rs) - Logs non-truncatable event data.
- [`ReloadSOL`](program/src/reload_sol.rs) - Reloads SOL mining rewards into automation.
- [`Reset`](program/src/reset.rs) - Resets the board for a new round.
- [`Close`](program/src/close.rs) - Closes an account.

#### Referrals
- [`CreateReferral`](program/src/create_referral.rs) - Creates a referral account.
- [`CreateReferralWithSession`](program/src/create_referral_with_session.rs) - Creates a referral account (Fogo session).
- [`ClaimReferral`](program/src/claim_referral.rs) - Claims referral rewards.
- [`ClaimReferralWithSession`](program/src/claim_referral_with_session.rs) - Claims referral rewards (Fogo session).

#### Auction
- [`PlaceBid`](program/src/place_bid.rs) - Places a bid on an auction well.
- [`PlaceBidWithSession`](program/src/place_bid_with_session.rs) - Places a bid on an auction well (Fogo session).
- [`ClaimAuctionOIL`](program/src/claim_auction_oil.rs) - Claims OIL rewards from auction mining.
- [`ClaimAuctionOILWithSession`](program/src/claim_auction_oil_with_session.rs) - Claims OIL rewards from auction mining (Fogo session).
- [`ClaimAuctionSOL`](program/src/claim_auction_sol.rs) - Claims SOL rewards from auction mining.
- [`ClaimAuctionSOLWithSession`](program/src/claim_auction_sol_with_session.rs) - Claims SOL rewards from auction mining (Fogo session).

#### Staking
- [`Deposit`](program/src/deposit.rs) - Deposits OIL into a stake account.
- [`DepositWithSession`](program/src/deposit_with_session.rs) - Deposits OIL into a stake account (Fogo session).
- [`Withdraw`](program/src/withdraw.rs) - Withdraws OIL from a stake account.
- [`WithdrawWithSession`](program/src/withdraw_with_session.rs) - Withdraws OIL from a stake account (Fogo session).
- [`ClaimYield`](program/src/claim_yield.rs) - Claims staking yield.
- [`ClaimYieldWithSession`](program/src/claim_yield_with_session.rs) - Claims staking yield (Fogo session).

#### Admin
- [`Barrel`](program/src/barrel.rs) - Executes a buy-and-barrel transaction.
- [`Buyback`](program/src/buyback.rs) - Executes a buyback transaction.
- [`Wrap`](program/src/wrap.rs) - Wraps SOL in the treasury for swap transactions.
- [`SetAdmin`](program/src/set_admin.rs) - Re-assigns the admin authority.
- [`SetFeeCollector`](program/src/set_fee_collector.rs) - Updates the fee collection address.
- [`SetAdminFee`](program/src/set_admin_fee.rs) - Updates the admin fee rate.
- [`SetSwapProgram`](program/src/set_swap_program.rs) - Updates the swap program address.
- [`SetVarAddress`](program/src/set_var_address.rs) - Updates the entropy variable address.
- [`NewVar`](program/src/new_var.rs) - Creates a new entropy variable.
- [`SetAuction`](program/src/set_auction.rs) - Configures auction parameters.
- [`CreateWhitelist`](program/src/create_whitelist.rs) - Creates a whitelist account.
- [`SetTgeTimestamp`](program/src/set_tge_timestamp.rs) - Sets the token generation event timestamp.
- [`Migrate`](program/src/migrate.rs) - Migrates program state.
- [`Liq`](program/src/liq.rs) - Executes liquidity operations.

## State
- [`Automation`](api/src/state/automation.rs) - Tracks automation configs.
- [`Auction`](api/src/state/auction.rs) - Tracks auction configuration and state.
- [`Bid`](api/src/state/bid.rs) - Tracks individual auction bids.
- [`Board`](api/src/state/board.rs) - Tracks the current round number and timestamps.
- [`Config`](api/src/state/config.rs) - Global program configs.
- [`Miner`](api/src/state/miner.rs) - Tracks a miner's game state.
- [`Pool`](api/src/state/pool.rs) - Tracks staking pool state.
- [`Referral`](api/src/state/referral.rs) - Tracks referral account state.
- [`Round`](api/src/state/round.rs) - Tracks the game state of a given round.
- [`Seeker`](api/src/state/seeker.rs) - Tracks whether a Seeker token has been claimed.
- [`Stake`](api/src/state/stake.rs) - Manages a user's staking activity.
- [`Treasury`](api/src/state/treasury.rs) - Mints, burns, and escrows OIL tokens.
- [`Well`](api/src/state/well.rs) - Tracks auction well state.
- [`Whitelist`](api/src/state/whitelist.rs) - Tracks whitelist entries. 


## Tests

To run the test suite, use the Solana toolchain: 

```
cargo test-sbf
```

For line coverage, use llvm-cov:

```
cargo llvm-cov
```