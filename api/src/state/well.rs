use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::well_pda;

use super::{OilAccount, Auction};

/// Well account (one per well)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Well {
    /// Well ID (0-3) - which well this is for
    pub well_id: u64,
    
    /// Current epoch ID (increments each auction: 0, 1, 2, 3, etc.)
    pub epoch_id: u64,
    
    /// Current bidder/owner (Pubkey::default() if unowned)
    pub current_bidder: Pubkey,
    
    /// Initial price for current epoch (in lamports)
    pub init_price: u64,
    
    /// Mining per second (MPS) - current mining rate (OIL per second, in atomic units)
    pub mps: u64,
    
    /// Epoch start time (timestamp when current epoch started)
    pub epoch_start_time: u64,
    
    /// Accumulated OIL mined by current owner (not yet claimed)
    pub accumulated_oil: u64,
    
    /// Last time accumulated_oil was updated
    pub last_update_time: u64,
    
    /// Number of halvings that have occurred (for rate calculation)
    pub halving_count: u64,
    
    /// Total OIL ever mined from this well (lifetime)
    pub lifetime_oil_mined: u64,
    
    /// Total OIL mined by current operator (doesn't reset when claimed, only when ownership changes)
    pub operator_total_oil_mined: u64,
    
    /// Buffer field (for future use) - previously is_pool_owned
    pub buffer_c: u64,
    
    /// Total contributed FOGO for current epoch (tracks native SOL balance in Well PDA's system account)
    /// Incremented on each contribution, decremented when pool bids
    /// Reset to 0 when epoch ends
    pub total_contributed: u64,
    
    /// Pool bid cost - stores the bid_amount when pool bids
    /// Used to calculate original_total when pool gets outbid
    /// Reset to 0 when epoch ends
    pub pool_bid_cost: u64,
}

impl Well {
    pub fn pda(well_id: u64) -> (Pubkey, u8) {
        well_pda(well_id)
    }

    pub fn current_price(&self, auction: &Auction, clock: &Clock) -> u64 {
        use crate::consts::AUCTION_FLOOR_PRICE;
        
        // If well has no owner (never been bid on), show starting price
        use solana_program::pubkey::Pubkey;
        if self.current_bidder == Pubkey::default() {
            return self.init_price; // Return starting price for unowned wells
        }
        
        let elapsed = clock.unix_timestamp.saturating_sub(self.epoch_start_time as i64);
        let duration = auction.auction_duration_seconds as i64;
        
        if elapsed >= duration {
            return AUCTION_FLOOR_PRICE; // Auction expired, price is at floor
        }
        
        // Linear decay: price = floor + (init_price - floor) * (remaining / duration)
        let remaining = duration - elapsed;
        let price_range = self.init_price.saturating_sub(AUCTION_FLOOR_PRICE);
        let decayed_amount = (price_range as u128 * remaining as u128 / duration as u128) as u64;
        AUCTION_FLOOR_PRICE + decayed_amount
    }

    /// Calculate the effective mining rate at a given point in time based on base rate and halvings
    fn calculate_rate_at_time(&self, base_rate: u64, halving_count_at_time: u64) -> u64 {
        if halving_count_at_time == 0 {
            return base_rate;
        }
        
        // Apply first halving (50% reduction)
        let mut rate = base_rate / 2;
        
        // Apply subsequent halvings (25% reduction each)
        for _ in 1..halving_count_at_time {
            rate = (rate * 3) / 4;
        }
        
        rate
    }
    
    /// Calculate how many halvings had occurred by a given timestamp
    /// 
    /// Halving schedule:
    /// - First halving: 14 days after initialization (at last_halving_time + FIRST_HALVING_PERIOD_SECONDS when halving_count = 0)
    /// - Subsequent halvings: Every 28 days after the first halving
    /// 
    /// When halving_count > 0, last_halving_time is when the most recent halving occurred.
    fn halving_count_at_time(auction: &Auction, timestamp: u64) -> u64 {
        if auction.halving_count == 0 {
            // No halvings have occurred yet
            // last_halving_time is the initialization time
            let first_halving_time = auction.last_halving_time + Auction::FIRST_HALVING_PERIOD_SECONDS;
            if timestamp < first_halving_time {
                return 0;
            }
            // First halving should have occurred but hasn't been applied yet
            // Return 0 to be safe - it will be applied when someone interacts
            return 0;
        }
        
        // Calculate when the first halving occurred
        // If halving_count = 1: first halving was at last_halving_time
        // If halving_count = 2: first halving was 28 days before last_halving_time
        // If halving_count = 3: first halving was 56 days before last_halving_time, etc.
        let first_halving_time = if auction.halving_count == 1 {
            auction.last_halving_time
        } else {
            // First halving was (halving_count - 1) * halving_period_seconds before the last halving
            auction.last_halving_time.saturating_sub(
                (auction.halving_count - 1) * auction.halving_period_seconds
            )
        };
        
        if timestamp < first_halving_time {
            return 0;
        }
        
        // Calculate how many halvings occurred by this timestamp
        // First halving is at first_halving_time, subsequent are every halving_period_seconds
        let time_since_first = timestamp.saturating_sub(first_halving_time);
        
        // First halving counts as 1, then add subsequent halvings (every halving_period_seconds)
        let halving_count = 1 + (time_since_first / auction.halving_period_seconds);
        
        // Cap at the actual halving_count (can't have more halvings than have occurred)
        halving_count.min(auction.halving_count)
    }

    pub fn update_accumulated_oil(&mut self, auction: &Auction, clock: &Clock) {
        // Skip if no owner
        use solana_program::pubkey::Pubkey;
        if self.current_bidder == Pubkey::default() {
            return;
        }
        
        let last_update = self.last_update_time as i64;
        let current_time = clock.unix_timestamp as u64;
        let elapsed = clock.unix_timestamp.saturating_sub(last_update);
        if elapsed <= 0 {
            return;
        }
        
        // Get base rate for this well (we need well_id, but we can derive it from self.well_id)
        let base_rate = auction.base_mining_rates[self.well_id as usize];
        
        // Calculate halving counts at start and end of period
        let halving_count_at_start = Self::halving_count_at_time(auction, last_update as u64);
        let halving_count_at_end = Self::halving_count_at_time(auction, current_time);
        
        // Calculate rate at start of period
        let rate_at_start = self.calculate_rate_at_time(base_rate, halving_count_at_start);
        
        // If no halving occurred during this period, use simple calculation
        if halving_count_at_start == halving_count_at_end {
            let oil_mined = rate_at_start.checked_mul(elapsed as u64).unwrap_or(0);
            self.accumulated_oil = self.accumulated_oil
                .checked_add(oil_mined)
                .unwrap_or(u64::MAX);
            self.lifetime_oil_mined = self.lifetime_oil_mined
                .checked_add(oil_mined)
                .unwrap_or(u64::MAX);
            self.operator_total_oil_mined = self.operator_total_oil_mined
                .checked_add(oil_mined)
                .unwrap_or(u64::MAX);
            self.last_update_time = current_time;
            return;
        }
        
        // Halving(s) occurred during this period - calculate in segments
        // Calculate when first halving occurred (needed for segment calculation)
        let first_halving_time = if auction.halving_count == 0 {
            auction.last_halving_time + Auction::FIRST_HALVING_PERIOD_SECONDS
        } else if auction.halving_count == 1 {
            auction.last_halving_time
        } else {
            auction.last_halving_time.saturating_sub(
                (auction.halving_count - 1) * auction.halving_period_seconds
            )
        };
        
        let mut total_oil = 0u64;
        let mut segment_start = last_update as u64;
        let mut current_halving_count = halving_count_at_start;
        
        while segment_start < current_time && current_halving_count < halving_count_at_end {
            // Calculate when the next halving occurred
            let next_halving_time = if current_halving_count == 0 {
                first_halving_time
            } else {
                // Subsequent halvings are every halving_period_seconds after the first
                first_halving_time + (current_halving_count as u64 * auction.halving_period_seconds)
            };
            
            let segment_end = next_halving_time.min(current_time);
            let segment_time = segment_end.saturating_sub(segment_start);
            let segment_rate = self.calculate_rate_at_time(base_rate, current_halving_count);
            
            total_oil = total_oil.checked_add(
                segment_rate.checked_mul(segment_time).unwrap_or(0)
            ).unwrap_or(u64::MAX);
            
            segment_start = segment_end;
            current_halving_count += 1;
        }
        
        // Calculate remaining time after all halvings in this period
        if segment_start < current_time {
            let remaining_time = current_time.saturating_sub(segment_start);
            let final_rate = self.calculate_rate_at_time(base_rate, halving_count_at_end);
            total_oil = total_oil.checked_add(
                final_rate.checked_mul(remaining_time).unwrap_or(0)
            ).unwrap_or(u64::MAX);
        }
        
        let oil_mined = total_oil;
        
        self.accumulated_oil = self.accumulated_oil
            .checked_add(oil_mined)
            .unwrap_or(u64::MAX);
        
        self.lifetime_oil_mined = self.lifetime_oil_mined
            .checked_add(oil_mined)
            .unwrap_or(u64::MAX);
        
        // Track total mined by current operator (persists even after claiming)
        self.operator_total_oil_mined = self.operator_total_oil_mined
            .checked_add(oil_mined)
            .unwrap_or(u64::MAX);
        
        self.last_update_time = current_time;
    }

    /// Apply all halvings that have already occurred (based on auction.halving_count)
    /// This is used when resetting well.mps to base rate in a new epoch
    /// 
    /// Safety: This function assumes well.mps has just been reset to base rate.
    /// It applies all halvings that have occurred according to auction.halving_count.
    pub fn apply_existing_halvings(&mut self, auction: &Auction) {
        if auction.halving_count == 0 {
            // No halvings have occurred yet, keep base rate
            self.halving_count = 0;
            return;
        }
        
        // Apply first halving (50% reduction)
        self.mps = self.mps / 2;
        
        // Apply subsequent halvings (25% reduction each)
        // auction.halving_count includes the first halving, so subtract 1 for subsequent halvings
        for _ in 0..(auction.halving_count - 1) {
            self.mps = (self.mps * 3) / 4; // 25% reduction (multiply by 0.75)
        }
        
        // Sync well's halving_count to match auction's
        self.halving_count = auction.halving_count;
    }

    pub fn check_and_apply_halving(&mut self, auction: &mut Auction, clock: &Clock) {
        // First, sync existing halvings if this well is behind
        // This ensures all wells stay in sync with the global halving state
        while self.halving_count < auction.halving_count {
            if self.halving_count == 0 {
                // Apply first halving (50% reduction)
                self.mps = self.mps / 2;
                self.halving_count = 1;
            } else {
                // Apply subsequent halvings (25% reduction each)
                self.mps = (self.mps * 3) / 4;
                self.halving_count += 1;
            }
        }
        
        // Then check if we should apply NEW halvings based on current time
        let current_time = clock.unix_timestamp as u64;
        let (halvings_to_apply, is_first_halving) = auction.should_apply_halving(current_time);
        
        if halvings_to_apply > 0 {
            if is_first_halving {
                // First halving: 50% reduction
                self.mps = self.mps / 2; // 50% reduction
                self.halving_count += 1;
                auction.halving_count += 1;
                auction.last_halving_time = current_time;
            } else {
                // Subsequent halvings: 25% reduction each (multiply by 0.75)
                for _ in 0..halvings_to_apply {
                    self.mps = (self.mps * 3) / 4; // 25% reduction (multiply by 0.75)
                    self.halving_count += 1;
                    auction.halving_count += 1;
                }
                // Update auction last_halving_time to current time
                auction.last_halving_time = current_time;
            }
        }
    }
}

account!(OilAccount, Well);

