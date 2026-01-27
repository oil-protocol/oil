use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_program::program_error::ProgramError;
use solana_program::log::sol_log;
use steel::*;

use super::OilAccount;
use crate::consts::REFERRAL;

/// Referral account tracks a referrer's stats and pending rewards.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Referral {
    /// The authority (wallet) of this referrer.
    pub authority: Pubkey,

    /// Total number of miners referred by this referrer.
    pub total_referred: u64,

    /// Total SOL earned from referrals (lifetime, for stats).
    pub total_sol_earned: u64,

    /// Total OIL earned from referrals (lifetime, for stats).
    pub total_oil_earned: u64,

    /// Pending SOL rewards to claim.
    pub pending_sol: u64,

    /// Pending OIL rewards to claim.
    pub pending_oil: u64,
}

impl Referral {
    pub fn claim_sol(&mut self) -> u64 {
        let amount = self.pending_sol;
        self.pending_sol = 0;
        amount
    }

    pub fn claim_oil(&mut self) -> u64 {
        let amount = self.pending_oil;
        self.pending_oil = 0;
        amount
    }

    pub fn credit_sol_referral(&mut self, total_amount: u64) -> u64 {
        // Calculate referral bonus (0.5% of total claim)
        let referral_amount = if total_amount > 0 {
            total_amount / 200 // 0.5% = 1/200
        } else {
            0
        };

        // Credit referral account
        if referral_amount > 0 {
            self.pending_sol += referral_amount;
            self.total_sol_earned += referral_amount;
        }

        referral_amount
    }

    pub fn credit_oil_referral(&mut self, total_amount: u64) -> u64 {
        // Calculate referral bonus (0.5% of total claim)
        let referral_amount = if total_amount > 0 {
            total_amount / 200 // 0.5% = 1/200
        } else {
            0
        };

        // Credit referral account
        if referral_amount > 0 {
            self.pending_oil += referral_amount;
            self.total_oil_earned += referral_amount;
        }

        referral_amount
    }

    pub fn process_new_miner_referral<'a>(
        referral_info_opt: Option<&AccountInfo<'a>>,
        referrer: Pubkey,
        authority: Pubkey,
    ) -> Result<(), ProgramError> {
        // Only process if referrer is valid (not default and not self-referral)
        if referrer == Pubkey::default() || referrer == authority {
            return Ok(());
        }
        
        // Referral account must be provided
        let referral_info = referral_info_opt.ok_or(ProgramError::NotEnoughAccountKeys)?;
        
        // Validate referral account
        referral_info
            .is_writable()?
            .has_seeds(&[REFERRAL, &referrer.to_bytes()], &crate::ID)?;
        
        // Referral account must exist
        if referral_info.data_is_empty() {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Increment total_referred
        let referral = referral_info.as_account_mut::<Referral>(&crate::ID)?;
        referral.total_referred += 1;
        sol_log(&format!("Referral: {} now has {} referrals", referrer, referral.total_referred));
        
        Ok(())
    }
}

account!(OilAccount, Referral);
