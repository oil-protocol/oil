use serde::{Deserialize, Serialize};
use solana_program::program_error::ProgramError;
use solana_program::log::sol_log;
use steel::*;

use crate::consts::WHITELIST;
use super::OilAccount;

/// Whitelist tracks access codes for pre-mine phase.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Whitelist {
    /// The code hash (first 32 bytes of keccak256 hash of the code string)
    pub code_hash: [u8; 32],
    
    /// Number of times this code has been used (optional tracking)
    pub usage_count: u64,
}

impl Whitelist {
    /// Derives the PDA for a Whitelist account.
    pub fn pda(code_hash: [u8; 32]) -> (Pubkey, u8) {
        use crate::ID;
        Pubkey::find_program_address(&[WHITELIST, &code_hash], &ID)
    }

    /// Validates and processes a premine access code for a new transaction.
    pub fn validate_premine_code<'a>(
        is_premine: bool,
        has_access_code: bool,
        access_code_hash: [u8; 32],
        whitelist_info_opt: Option<&AccountInfo<'a>>,
    ) -> Result<(), ProgramError> {
        if is_premine {
            if !has_access_code {
                return Err(ProgramError::InvalidArgument); // Access code required during pre-mine
            }
            
            // Validate access code: check if Whitelist PDA exists
            let whitelist_info = whitelist_info_opt.ok_or(ProgramError::NotEnoughAccountKeys)?;
            
            // Derive PDA using code_hash only (not tied to wallet)
            let (whitelist_pda, _) = Self::pda(access_code_hash);
            whitelist_info.has_address(&whitelist_pda)?;
            
            // Check if account exists (has data)
            if whitelist_info.data_is_empty() {
                return Err(ProgramError::InvalidArgument); // Access code not found
            }
            
            // Validate the code hash matches
            let whitelist = whitelist_info.as_account::<Whitelist>(&crate::ID)?;
            if whitelist.code_hash != access_code_hash {
                return Err(ProgramError::InvalidArgument); // Access code hash mismatch
            }
            
            // Increment usage count (optional tracking)
            let whitelist_mut = whitelist_info.as_account_mut::<Whitelist>(&crate::ID)?;
            whitelist_mut.usage_count = whitelist_mut.usage_count.saturating_add(1);
            
            sol_log(&format!(
                "Pre-mine: access code validated, usage_count={}",
                whitelist_mut.usage_count
            ));
        } else if has_access_code {
            // Access code provided but not in pre-mine - ignore it (not an error)
            sol_log("Access code provided but pre-mine is not active - ignoring");
        }
        
        Ok(())
    }
}

account!(OilAccount, Whitelist);
