use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::share_pda;

use super::OilAccount;

/// Share account tracks a user's contribution to a specific epoch for a specific well
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Share {
    /// Authority who made the contribution
    pub authority: Pubkey,
    
    /// Well ID this share is for (0-3)
    pub well_id: u64,
    
    /// Epoch ID this share is for (included in PDA, stored here for convenience)
    pub epoch_id: u64,
    
    /// User's contribution to this epoch's pool (in lamports)
    pub contribution: u64,
    
    /// Timestamp when share was created (first contribution to this epoch)
    pub created_at: u64,
    
    /// Amount of OIL claimed from this epoch (0 = not checkpointed, >0 = checkpointed)
    pub claimed_oil: u64,
    
    /// Amount of SOL refund claimed from this epoch
    pub claimed_sol: u64,
    
    /// Buffer field for future extensions
    pub buffer_a: u64,
    
    /// Buffer field for future extensions
    pub buffer_b: u64,
    
    /// Buffer field for future extensions
    pub buffer_c: u64,
}

impl Share {
    pub fn pda(authority: Pubkey, well_id: u64, epoch_id: u64) -> (Pubkey, u8) {
        share_pda(authority, well_id, epoch_id)
    }

    pub fn initialize(&mut self, authority: Pubkey, well_id: u64, epoch_id: u64, clock: &Clock) {
        self.authority = authority;
        self.well_id = well_id;
        self.epoch_id = epoch_id;
        self.contribution = 0;
        self.created_at = clock.unix_timestamp as u64;
        self.claimed_oil = 0;
        self.claimed_sol = 0;
        self.buffer_a = 0;
        self.buffer_b = 0;
        self.buffer_c = 0;
    }
}

account!(OilAccount, Share);
