use oil_api::prelude::*;
use steel::*;

/// Creates a new var account.
pub fn process_new_var(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = NewVar::try_from_bytes(data)?;
    let id = u64::from_le_bytes(args.id);
    let commit = args.commit;
    let samples = u64::from_le_bytes(args.samples);

    // Load accounts.
    let [signer_info, board_info, config_info, provider_info, var_info, system_program, entropy_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    signer_info.is_signer()?;
    let board = board_info.as_account_mut::<Board>(&oil_api::ID)?;
    
    config_info
        .as_account_mut::<Config>(&oil_api::ID)?
        .assert_mut_err(
            |c| c.admin == *signer_info.key,
            OilError::NotAuthorized.into(),
        )?;
    
    entropy_program.is_program(&entropy_rng_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Get var PDA to verify address matches
    let (var_pda_address, _var_bump) = entropy_rng_api::state::var_pda(*board_info.key, id);
    
    // Verify var_info matches expected PDA
    if *var_info.key != var_pda_address {
        return Err(ProgramError::InvalidSeeds);
    }

    let clock = Clock::get()?;
    let end_at = if board.end_slot == u64::MAX || board.end_slot <= clock.slot {
        // Round hasn't started or has ended - use a future slot with buffer
        // Add extra buffer to account for slot advancement between parent and child program execution
        clock.slot + 200
    } else {
        // Use the current round's end slot, but ensure it's still in the future
        board.end_slot.max(clock.slot + 200)
    };

    let open_ix = entropy_rng_api::sdk::open(
            *board_info.key,
            *signer_info.key,
            id,
            *provider_info.key,
            commit,
            false,
            samples,
        end_at,
    );

    // The third parameter to invoke_signed is the program that owns the PDA seeds
    invoke_signed(
        &open_ix,
        &[
            board_info.clone(),
            signer_info.clone(),
            provider_info.clone(),
            var_info.clone(),
            system_program.clone(),
        ],
        &oil_api::ID,
        &[BOARD],
    )?;

    Ok(())
}