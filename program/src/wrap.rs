use oil_api::prelude::*;
use solana_program::rent::Rent;
use steel::*;

/// Send SOL from the treasury to the WSOL account.
/// 
/// Can wrap any amount from either treasury.balance or treasury.liquidity.
/// use_liquidity: 0 = use balance, 1 = use liquidity
/// amount: the amount to wrap (in lamports). If 0, wraps all available.
pub fn process_wrap(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Wrap::try_from_bytes(data)?;
    let use_liquidity = args.use_liquidity != 0;
    let requested_amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, _config_info, treasury_info, treasury_sol_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&BURY_AUTHORITY)?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    treasury_sol_info
        .is_writable()?
        .as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    system_program.is_program(&solana_program::system_program::ID)?;

    // Determine amount to wrap:
    // - If use_liquidity is true, use treasury.liquidity
    // - Otherwise, use treasury.balance
    // - If amount is 0, wrap all available
    // - Otherwise, wrap the specified amount (capped at available)
    let wrap_amount = if use_liquidity {
        if requested_amount == 0 {
            // Wrap all available liquidity
            treasury.liquidity.min(treasury.balance)
        } else {
            // Wrap specified amount from liquidity
            requested_amount.min(treasury.liquidity).min(treasury.balance)
        }
    } else {
        if requested_amount == 0 {
            // Wrap all available balance
            treasury.balance
        } else {
            // Wrap specified amount from balance
            requested_amount.min(treasury.balance)
        }
    };
    assert!(wrap_amount > 0, "No SOL available to wrap");

    // Send SOL to the WSOL account.
    treasury_info.send(wrap_amount, treasury_sol_info);

    // Check min balance.
    let min_balance = Rent::get()?.minimum_balance(std::mem::size_of::<Treasury>());
    assert!(
        treasury_info.lamports() >= min_balance,
        "Insufficient SOL balance"
    );

    // Update treasury.
    treasury.balance -= wrap_amount;
    
    // If using liquidity, also decrement liquidity tracking
    if use_liquidity {
        treasury.liquidity = treasury.liquidity.saturating_sub(wrap_amount);
    }

    Ok(())
}
