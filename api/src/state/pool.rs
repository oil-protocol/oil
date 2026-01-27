use serde::{Deserialize, Serialize};
use steel::*;

use super::OilAccount;

/// Pool account holds all staking-related data and SOL rewards for stakers.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Pool {
    /// The amount of SOL available for staking rewards (2% of round winnings).
    pub balance: u64,

    /// The cumulative SOL distributed to stakers, divided by the total stake score at the time of distribution.
    pub stake_rewards_factor: Numeric,

    /// The current total staked score (sum of all balance * multiplier).
    pub total_staked_score: u64,

    /// The current total amount of OIL staked (stakers earn SOL rewards, not OIL).
    pub total_staked: u64,

    /// Buffer field (for future use, e.g., OIL rewards).
    pub buffer_a: Numeric,

    /// Total amount of OIL burned from early withdrawal penalties (deflationary).
    pub total_burned_penalties: u64,

    /// Buffer field (for future use).
    pub buffer_c: u64,
}

account!(OilAccount, Pool);

