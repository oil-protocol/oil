use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::micro_pda;

use super::OilAccount;

/// Micro account stores per-epoch totals for a specific well and epoch
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Micro {
    /// Well ID (0-3)
    pub well_id: u64,
    
    /// Epoch ID
    pub epoch_id: u64,
    
    /// Total pooled FOGO for this epoch (original total before pool bid deduction)
    pub total_contributed: u64,
    
    /// Total OIL mined during this epoch
    pub total_oil_mined: u64,
    
    /// Total refund when outbid (86% of new bid + leftover FOGO)
    pub total_refund: u64,
    
    /// Number of unique contributors (optional, for stats)
    pub pool_members: u64,
    
    /// Buffer field for future extensions
    pub buffer_a: u64,
    
    /// Buffer field for future extensions
    pub buffer_b: u64,
    
    /// Buffer field for future extensions
    pub buffer_c: u64,
}

impl Micro {
    pub fn pda(well_id: u64, epoch_id: u64) -> (Pubkey, u8) {
        micro_pda(well_id, epoch_id)
    }
}

account!(OilAccount, Micro);
