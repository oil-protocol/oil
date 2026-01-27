use oil_api::prelude::*;
use solana_program::log::sol_log;
use solana_program::native_token::lamports_to_sol;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Swap vaulted SOL to OIL, and burn the OIL.
pub fn process_buyback(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Load accounts.
    let (oil_accounts, swap_accounts) = accounts.split_at(9);
    let [signer_info, board_info, _config_info, mint_info, treasury_info, treasury_oil_info, treasury_sol_info, token_program, oil_program] =
        oil_accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?.has_address(&BURY_AUTHORITY)?;
    board_info.as_account_mut::<Board>(&oil_api::ID)?;
    let oil_mint = mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    let treasury_oil =
        treasury_oil_info.as_associated_token_account(treasury_info.key, &MINT_ADDRESS)?;
    treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    token_program.is_program(&spl_token::ID)?;
    oil_program.is_program(&oil_api::ID)?;

    // Sync native token balance.
    sync_native(treasury_sol_info)?;

    // Record pre-swap balances.
    let treasury_sol =
        treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    let pre_swap_oil_balance = treasury_oil.amount();
    let pre_swap_sol_balance = treasury_sol.amount();
    assert!(pre_swap_sol_balance > 0);

    // Record pre-swap mint supply.
    let pre_swap_mint_supply = oil_mint.supply();

    // Record pre-swap treasury lamports.
    let pre_swap_treasury_lamports = treasury_info.lamports();

    // Build swap accounts.
    let accounts: Vec<AccountMeta> = swap_accounts
        .iter()
        .map(|acc| {
            let is_signer = if acc.key == treasury_info.key {
                true // Our program signs for treasury via invoke_signed
            } else {
                acc.is_signer().ok().is_some()
            };
            // Check if account is writable - TypeScript has set this based on Valiant's roles
            let is_writable = acc.is_writable().ok().is_some();
            AccountMeta {
                pubkey: *acc.key,
                is_signer,
                is_writable,
            }
        })
        .collect();

    // Build swap accounts infos.
    let accounts_infos: Vec<AccountInfo> = swap_accounts
        .iter()
        .map(|acc| AccountInfo { ..acc.clone() })
        .collect();

    // Invoke swap program.
    invoke_signed(
        &Instruction {
            program_id: SWAP_PROGRAM,
            accounts,
            data: data.to_vec(),
        },
        &accounts_infos,
        &oil_api::ID,
        &[TREASURY],
    )?;

    // Record post-swap treasury lamports.
    let post_swap_treasury_lamports = treasury_info.lamports();
    assert_eq!(
        post_swap_treasury_lamports, pre_swap_treasury_lamports,
        "Treasury lamports changed during swap: {} -> {}",
        pre_swap_treasury_lamports, post_swap_treasury_lamports
    );

    // Record post-swap mint supply.
    let post_swap_mint_supply = mint_info.as_mint()?.supply();
    assert_eq!(
        post_swap_mint_supply, pre_swap_mint_supply,
        "Mint supply changed during swap: {} -> {}",
        pre_swap_mint_supply, post_swap_mint_supply
    );

    // Record post-swap balances.
    let treasury_oil =
        treasury_oil_info.as_associated_token_account(treasury_info.key, &MINT_ADDRESS)?;
    let treasury_sol =
        treasury_sol_info.as_associated_token_account(treasury_info.key, &SOL_MINT)?;
    let post_swap_oil_balance = treasury_oil.amount();
    let post_swap_sol_balance = treasury_sol.amount();
    let total_oil = post_swap_oil_balance - pre_swap_oil_balance;
    assert_eq!(post_swap_sol_balance, 0);
    assert!(post_swap_oil_balance >= pre_swap_oil_balance);
    sol_log(
        &format!(
            "📈 Swapped {} SOL into {} OIL",
            lamports_to_sol(pre_swap_sol_balance),
            amount_to_ui_amount(total_oil, TOKEN_DECIMALS),
        )
        .as_str(),
    );

    // Burn OIL (no sharing with stakers - stakers earn SOL rewards from round winnings, not OIL).
    let burn_amount = total_oil;
    burn_signed(
        treasury_oil_info,
        mint_info,
        treasury_info,
        token_program,
        burn_amount,
        &[TREASURY],
    )?;

    // Update total_barrelled
    treasury.total_barrelled = treasury.total_barrelled.saturating_add(burn_amount);

    sol_log(
        &format!(
            "🔥 Barreled {} OIL",
            amount_to_ui_amount(burn_amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );

    // Emit event.
    let mint = mint_info.as_mint()?;
    program_log(
        &[board_info.clone(), oil_program.clone()],
        BarrelEvent {
            disc: 1,
            oil_barreled: burn_amount,
            oil_shared: 0, // No longer sharing OIL - all burned for deflation
            sol_amount: pre_swap_sol_balance,
            new_circulating_supply: mint.supply(),
            ts: Clock::get()?.unix_timestamp,
        }
        .to_bytes(),
    )?;

    Ok(())
}