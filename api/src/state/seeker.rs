use steel::*;

use super::OilAccount;

/// Seeker is an account which prevents multiple Seeker genesis tokens from being claimed.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Seeker {
    // The mint address of the Seeker token.
    pub mint: Pubkey,
}

account!(OilAccount, Seeker);