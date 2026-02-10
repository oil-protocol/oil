use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::auction_pda;
use super::OilAccount;

/// Singleton auction configuration account
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Auction {
    /// Subsequent halving period in seconds (28 days = 2,419,200 seconds)
    pub halving_period_seconds: u64,
    
    /// Timestamp of the last halving event (Unix timestamp in seconds)
    pub last_halving_time: u64,
    
    /// Base mining rates per well (OIL per second, in atomic units)
    pub base_mining_rates: [u64; 4],
    
    /// Auction duration in seconds (1 hour = 3600)
    pub auction_duration_seconds: u64,
    
    /// Starting prices per well (in lamports)
    pub starting_prices: [u64; 4],
    
    /// Buffer field (for future use)
    pub buffer_a: Numeric,
    
    /// Number of halvings that have occurred (0 = none, 1 = first 50% halving, 2+ = 25% halvings)
    pub halving_count: u64,
    
    /// Buffer field (for future use)
    pub buffer_c: u64,
    
    /// Buffer field (for future use)
    pub buffer_d: u64,
}

impl Auction {
    pub fn pda() -> (Pubkey, u8) {
        auction_pda()
    }

    /// First halving period in seconds (14 days = 1,209,600 seconds)
    /// This is a constant, not stored in the account
    pub const FIRST_HALVING_PERIOD_SECONDS: u64 = 14 * 24 * 60 * 60;

    /// Get the timestamp when the next halving should occur
    pub fn next_halving_time(&self) -> u64 {
        if self.halving_count == 0 {
            // First halving is at FIRST_HALVING_PERIOD_SECONDS (14 days) after initialization
            // last_halving_time is set to initialization time, so add 14 days
            return self.last_halving_time + Self::FIRST_HALVING_PERIOD_SECONDS;
        }
        
        // After first halving, subsequent halvings are every halving_period_seconds (28 days)
        self.last_halving_time + self.halving_period_seconds
    }
    
    /// Check if halving should be applied based on current time
    /// Returns (halvings_to_apply, is_first_halving)
    /// 
    /// Schedule:
    /// - First halving: 14 days after initialization (50% reduction)
    /// - Subsequent halvings: Every 28 days after first halving (25% reduction each)
    pub fn should_apply_halving(&self, current_time: u64) -> (u64, bool) {
        // If no halvings have occurred yet, check for first halving (14 days)
        if self.halving_count == 0 {
            // last_halving_time is set to initialization time, so check if 14 days have passed
            if current_time >= self.last_halving_time + Self::FIRST_HALVING_PERIOD_SECONDS {
                return (1, true);
            }
            return (0, false);
        }
        
        // After first halving, check for subsequent halvings (every 28 days)
        if current_time < self.last_halving_time {
            return (0, false); // Time hasn't reached last halving yet
        }
        
        // Calculate how many halving periods have passed since last halving
        let time_since_last_halving = current_time - self.last_halving_time;
        let halvings_to_apply = time_since_last_halving / self.halving_period_seconds;
        
        // After first halving, all subsequent halvings are 25% reductions
        (halvings_to_apply, false)
    }
}

account!(OilAccount, Auction);

