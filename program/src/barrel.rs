use oil_api::prelude::*;
use solana_program::log::sol_log;
use solana_program::pubkey;
use spl_token::amount_to_ui_amount;
use steel::*;

const LIQ_MANAGER: Pubkey = pubkey!("DEvGq2WVuA3qkSCtwwuMYThY4onkJunEHSAxU5cieph8");

/// Barrel (burn) leftover OIL from the liq manager.
/// Burns 100% of the OIL.
pub fn process_barrel(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Barrel::try_from_bytes(data)?;
    let requested_amount = u64::from_le_bytes(args.amount);

    // Load accounts.
    let [signer_info, sender_info, board_info, mint_info, treasury_info, treasury_oil_info, token_program, oil_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&LIQ_MANAGER)?;
    let sender = sender_info
        .is_writable()?
        .as_associated_token_account(signer_info.key, &MINT_ADDRESS)?;
    board_info.as_account_mut::<Board>(&oil_api::ID)?;
    mint_info.has_address(&MINT_ADDRESS)?; // Verify mint address
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    treasury_oil_info.as_associated_token_account(treasury_info.key, &MINT_ADDRESS)?;
    token_program.is_program(&spl_token::ID)?;
    oil_program.is_program(&oil_api::ID)?;

    // Determine amount to barrel (use all if amount is 0, otherwise use requested amount)
    let amount = if requested_amount == 0 {
        sender.amount()
    } else {
        sender.amount().min(requested_amount)
    };
    assert!(amount > 0, "No OIL to barrel");

    // Transfer OIL from sender (LIQ_MANAGER) to treasury.
    transfer(
        signer_info,
        sender_info,
        treasury_oil_info,
        token_program,
        amount,
    )?;

    // Burn all OIL (no sharing with stakers).
    let burn_amount = amount;
    burn_signed(
        treasury_oil_info,
        mint_info,
        treasury_info,
        token_program,
        burn_amount,
        &[TREASURY],
    )?;

    sol_log(
        &format!(
            "🔥 Barreled {} OIL",
            amount_to_ui_amount(burn_amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );

    // Update total_barrelled
    treasury.total_barrelled = treasury.total_barrelled.saturating_add(burn_amount);

    // Emit event.
    let mint = mint_info.as_mint()?;
    program_log(
        &[board_info.clone(), oil_program.clone()],
        BarrelEvent {
            disc: 1,
            oil_barreled: burn_amount,
            oil_shared: 0, // No sharing with stakers
            sol_amount: 0, // No SOL involved in barrel from liq manager
            new_circulating_supply: mint.supply(),
            ts: Clock::get()?.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}
