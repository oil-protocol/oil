use oil_api::prelude::*;
use solana_program::log::sol_log;
use solana_program::native_token::lamports_to_sol;
use solana_program::pubkey;
use steel::*;

const LIQ_MANAGER: Pubkey = pubkey!("DEvGq2WVuA3qkSCtwwuMYThY4onkJunEHSAxU5cieph8");

/// Send wrapped SOL from the treasury to the liq manager.
/// The liq manager (off-chain) will handle adding liquidity to the pool.
pub fn process_liq(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, board_info, _config_info, manager_info, manager_sol_info, treasury_info, treasury_sol_info, token_program, oil_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&BURY_AUTHORITY)?;
    board_info.as_account_mut::<Board>(&oil_api::ID)?;
    manager_info.has_address(&LIQ_MANAGER)?;
    manager_sol_info
        .is_writable()?
        .as_associated_token_account(manager_info.key, &SOL_MINT)?;
    treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    token_program.is_program(&spl_token::ID)?;
    oil_program.is_program(&oil_api::ID)?;

    // Sync native token balance.
    sync_native(treasury_sol_info)?;

    // Record pre-transfer balance.
    let treasury_sol =
        treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    let liq_amount = treasury_sol.amount();
    assert!(liq_amount > 0, "No wrapped SOL available for liquidity");

    // Transfer wrapped SOL to liq manager.
    transfer_signed(
        treasury_info,
        treasury_sol_info,
        manager_sol_info,
        token_program,
        liq_amount,
        &[TREASURY],
    )?;

    // Verify transfer completed.
    let treasury_sol =
        treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    assert_eq!(treasury_sol.amount(), 0, "Transfer did not complete");
    
    sol_log(&format!("💦 Sent {} SOL to liq manager", lamports_to_sol(liq_amount)));

    // Emit event.
    program_log(
        &[board_info.clone(), oil_program.clone()],
        LiqEvent {
            disc: 3,
            sol_amount: liq_amount,
            recipient: *manager_info.key,
            ts: Clock::get()?.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
