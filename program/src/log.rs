use oil_api::prelude::*;
use steel::*;

/// No-op, use instruction data for logging w/o truncation.
pub fn process_log(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    let account_size = signer_info.data_len();
    
    if account_size == 40 {
        // Try Board account - valid for block-based logging
        if signer_info.as_account::<Board>(&oil_api::ID).is_ok() {
            return Ok(());
        }
    } else if account_size == 136 {
        // Try Auction account - valid for auction-based logging
        if signer_info.as_account::<Auction>(&oil_api::ID).is_ok() {
            return Ok(());
        }
    }
    
    // Neither Board nor Auction - invalid
    Err(ProgramError::InvalidAccountData)
}
