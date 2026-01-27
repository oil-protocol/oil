use serde::{Deserialize, Serialize};
use steel::*;

use super::OilAccount;

/// Treasury is a singleton account which is the mint authority for the OIL token and the authority of
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Treasury {
    // The amount of SOL collected for buy-barrel operations.
    pub balance: u64,

    /// The amount of SOL in the gusher rewards pool.
    pub gusher_sol: u64,

    /// The cumulative OIL distributed to miners, divided by the total unclaimed OIL at the time of distribution.
    pub block_rewards_factor: Numeric,

    /// Buffer field (previously stake_rewards_factor, now in Pool).
    pub buffer_a: Numeric,

    /// The total amount of OIL barreled (burned) through buyback operations.
    pub total_barrelled: u64,

    /// The current total amount of refined OIL mining rewards.
    pub block_total_refined: u64,

    /// The total amount of SOL held in treasury for auction rewards (to be claimed by miners).
    pub auction_rewards_sol: u64,

    /// The current total amount of unclaimed OIL mining rewards.
    pub block_total_unclaimed: u64,

    /// Auction-based mining: The cumulative OIL distributed to miners, divided by the total unclaimed auction OIL at the time of distribution.
    pub auction_rewards_factor: Numeric,

    /// Auction-based mining: The current total amount of unclaimed auction OIL mining rewards.
    pub auction_total_unclaimed: u64,

    /// Auction-based mining: The current total amount of refined auction OIL mining rewards.
    pub auction_total_refined: u64,

    /// The amount of SOL used for liquidity & market making
    pub liquidity: u64,
}

impl Treasury {
    pub fn credit_auction_rewards_sol(&mut self, amount: u64) {
        self.auction_rewards_sol += amount;
    }
}

account!(OilAccount, Treasury);