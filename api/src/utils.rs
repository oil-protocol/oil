use solana_program::program_error::ProgramError;
use solana_program::program::invoke;
use solana_program::sysvar::clock::Clock;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use steel::*;
use crate::state::Config;

/// Macro to extract accounts from an iterator with concise syntax.
/// 
/// Usage: `extract_accounts!(accounts_iter, [s, a, ps, pay, ...])`
/// 
/// This reduces repetitive `accounts_iter.next().unwrap()` calls.
#[macro_export]
macro_rules! extract_accounts {
    ($iter:expr, [$($var:ident),*]) => {
        $(
            let $var = $iter.next().unwrap();
        )*
    };
}

/// Creates a wrapped SOL ATA if it doesn't exist, otherwise validates it.
pub fn create_or_validate_wrapped_sol_ata<'a>(
    ata_info: &AccountInfo<'a>,
    owner_info: &AccountInfo<'a>,
    mint_info: &AccountInfo<'a>,
    payer_info: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    ata_program: &AccountInfo<'a>,
    _log_message: Option<&str>,
) -> Result<(), ProgramError> {
    ata_info.is_writable()?;
    
    // CRITICAL: Check if account is already owned by token program FIRST
    // If it is, it was created by a previous instruction (e.g., session wrap)
    // and we must NOT try to create it again (this causes "Transfer: from must not carry data" error)
    let is_empty = ata_info.data_is_empty();
    // Only check ownership if account is not empty (has_owner fails on empty accounts)
    let is_owned_by_token_program = !is_empty && ata_info.has_owner(token_program.key).is_ok();
    
    if is_owned_by_token_program {
        // Account is owned by token program - it exists, just validate it
        // Don't try to create it, even if validation initially fails
        ata_info.as_associated_token_account(owner_info.key, mint_info.key).map(|_| ())
    } else if is_empty {
        // Account doesn't exist - create it using idempotent instruction
        let create_ix = create_associated_token_account_idempotent(
            payer_info.key,
            owner_info.key,
            mint_info.key,
            token_program.key,
        );
        
        invoke(
            &create_ix,
            &[
                payer_info.clone(),
                ata_info.clone(),
                owner_info.clone(),
                mint_info.clone(),
                system_program.clone(),
                token_program.clone(),
                ata_program.clone(),
            ],
        )?;
        
        // After creation, validate to ensure it's correct
        ata_info.as_associated_token_account(owner_info.key, mint_info.key)?;
        Ok(())
    } else {
        // Account has data but is not owned by token program
        // Try to validate - if it fails, it's an error
        ata_info.as_associated_token_account(owner_info.key, mint_info.key).map(|_| ())
    }
}

/// Checks if pre-mine phase is currently active.
/// 
/// Pre-mine is active when:
/// - `config.tge_timestamp > 0` (TGE timestamp is set)
/// - `clock.unix_timestamp < config.tge_timestamp` (current time is before TGE)
/// 
/// Returns `true` if pre-mine is active, `false` otherwise.
pub fn is_premine_active(config: &Config, clock: &Clock) -> bool {
    config.tge_timestamp > 0 && clock.unix_timestamp < config.tge_timestamp
}
