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
    
    /// Buffer field (for future use)
    pub buffer_d: u64,
    
    /// Buffer field (for future use)
    pub buffer_e: u64,
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

    pub fn update_accumulated_oil(&mut self, clock: &Clock) {
        // Skip if no owner
        use solana_program::pubkey::Pubkey;
        if self.current_bidder == Pubkey::default() {
            return;
        }
        
        let last_update = self.last_update_time as i64;
        let elapsed = clock.unix_timestamp.saturating_sub(last_update);
        if elapsed <= 0 {
            return;
        }
        
        // Calculate OIL mined: rate * time
        let oil_mined = self.mps
            .checked_mul(elapsed as u64)
            .unwrap_or(0);
        
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
        
        self.last_update_time = clock.unix_timestamp as u64;
    }

    pub fn check_and_apply_halving(&mut self, auction: &mut Auction, clock: &Clock) {
        // Check if we should apply halvings based on current time
        let current_time = clock.unix_timestamp as u64;
        let halvings_to_apply = auction.should_apply_halving(current_time);
        
        if halvings_to_apply > 0 {
            // Apply halvings: 50% reduction (multiply by 0.5) per halving
            for _ in 0..halvings_to_apply {
                self.mps = self.mps / 2; // 50% reduction (multiply by 0.5)
                self.halving_count += 1;
            }
            
            // Update auction last_halving_time to current time
            auction.last_halving_time = current_time;
        }
    }
}

account!(OilAccount, Well);

