use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::rig_pda;

use super::OilAccount;

/// Rig account tracks a user's auction participation and checkpoint status across all wells
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Rig {
    /// Authority (user's wallet)
    pub authority: Pubkey,
    
    /// Last epoch participated in per well (index = well_id, 0-3)
    pub current_epoch_id: [u64; 4],
    
    /// Last epoch checkpointed per well (index = well_id, 0-3)
    pub checkpointed_epoch_id: [u64; 4],
    
    /// Buffer field for future extensions
    pub buffer_a: u64,
    
    /// Buffer field for future extensions
    pub buffer_b: u64,
    
    /// Buffer field for future extensions
    pub buffer_c: u64,
}

impl Rig {
    pub fn pda(authority: Pubkey) -> (Pubkey, u8) {
        rig_pda(authority)
    }

    pub fn initialize(&mut self, authority: Pubkey) {
        self.authority = authority;
        self.current_epoch_id = [0; 4];
        self.checkpointed_epoch_id = [0; 4];
        self.buffer_a = 0;
        self.buffer_b = 0;
        self.buffer_c = 0;
    }
}

account!(OilAccount, Rig);
