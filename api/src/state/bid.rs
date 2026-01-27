use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::bid_pda;

use super::OilAccount;

/// Bid account tracks individual user contributions to auction pools for a specific epoch
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Bid {
    /// Authority who made the contribution
    pub authority: Pubkey,
    
    /// Well ID this bid is for (0-3)
    pub well_id: u64,
    
    /// Epoch ID this bid is for (included in PDA, stored here for convenience)
    pub epoch_id: u64,
    
    /// Total amount of SOL contributed to this epoch's pool (in lamports)
    pub contribution: u64,
    
    /// Timestamp when bid was created (first contribution to this epoch)
    pub created_at: u64,
    
    /// Buffer field (for future use)
    pub buffer_a: u64,
    
    /// Buffer field (for future use)
    pub buffer_b: u64,
}

impl Bid {
    pub fn pda(authority: Pubkey, well_id: u64, epoch_id: u64) -> (Pubkey, u8) {
        bid_pda(authority, well_id, epoch_id)
    }
}

account!(OilAccount, Bid);

