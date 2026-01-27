use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::auction_pda;
use super::OilAccount;

/// Singleton auction configuration account
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Auction {
    /// Halving period in seconds (28 days = 2,419,200 seconds)
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
    
    /// Buffer field (for future use) - previously min_pool_contribution
    pub buffer_b: u64,
    
    /// Buffer field (for future use)
    pub buffer_c: u64,
    
    /// Buffer field (for future use)
    pub buffer_d: u64,
}

impl Auction {
    pub fn pda() -> (Pubkey, u8) {
        auction_pda()
    }

    /// Get the timestamp when the next halving should occur
    pub fn next_halving_time(&self) -> u64 {
        if self.last_halving_time == 0 {
            return self.halving_period_seconds;
        }
        self.last_halving_time + self.halving_period_seconds
    }
    
    /// Check if halving should be applied based on current time    
    pub fn should_apply_halving(&self, current_time: u64) -> u64 {
        if self.last_halving_time == 0 {
            // First halving check - if current_time >= halving_period_seconds, apply one halving
            if current_time >= self.halving_period_seconds {
                return 1;
            }
            return 0;
        }
        
        if current_time < self.last_halving_time {
            return 0; // Time hasn't reached last halving yet
        }
        
        // Calculate how many halving periods have passed
        let time_since_last_halving = current_time - self.last_halving_time;
        let halvings_to_apply = time_since_last_halving / self.halving_period_seconds;
        
        halvings_to_apply
    }
}

account!(OilAccount, Auction);

