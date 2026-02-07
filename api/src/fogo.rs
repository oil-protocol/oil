use fogo_sessions_sdk::session::is_session;
use fogo_sessions_sdk::token::PROGRAM_SIGNER_SEED;
use spl_token::instruction::close_account;
use solana_program::program::invoke_signed as solana_invoke_signed;
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use steel::*;
use crate::ID;

pub fn validate_program_signer(program_signer_info: &AccountInfo) -> Result<u8, ProgramError> {
    let (program_signer_pda, bump) = Pubkey::find_program_address(
        &[PROGRAM_SIGNER_SEED],
        &ID,
    );
    if program_signer_info.key != &program_signer_pda {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(bump)
}

pub fn validate_session(signer_info: &AccountInfo) -> Result<(), ProgramError> {
    if !is_session(signer_info) {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub fn transfer_wrapped_sol_and_unwrap<'a>(
    signer_info: &'a AccountInfo<'a>,
    program_signer_info: &'a AccountInfo<'a>,
    _payer_info: &'a AccountInfo<'a>,
    amount: u64,
    user_wrapped_sol_info: &'a AccountInfo<'a>,
    destination_wrapped_sol_info: &'a AccountInfo<'a>,
    destination_pda_info: &'a AccountInfo<'a>,
    mint_info: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    destination_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let bump = validate_program_signer(program_signer_info)?;
    
    let transfer_ix = fogo_sessions_sdk::token::instruction::transfer_checked(
        token_program.key,
        user_wrapped_sol_info.key,
        mint_info.key,
        destination_wrapped_sol_info.key,
        signer_info.key,
        Some(program_signer_info.key),
        amount,
        9,
    )?;
    
    solana_invoke_signed(
        &transfer_ix,
        &[
            user_wrapped_sol_info.clone(),
            mint_info.clone(),
            destination_wrapped_sol_info.clone(),
            signer_info.clone(),
            token_program.clone(),
            program_signer_info.clone(),
        ],
        &[&[PROGRAM_SIGNER_SEED, &[bump]]],
    )?;
    
    let close_ix = close_account(
        token_program.key,
        destination_wrapped_sol_info.key,
        destination_pda_info.key,
        destination_pda_info.key,
        &[],
    )?;
    
    invoke_signed(
        &close_ix,
        &[
            destination_wrapped_sol_info.clone(),
            destination_pda_info.clone(),
            destination_pda_info.clone(),
            token_program.clone(),
        ],
        &ID,
        destination_seeds,
    )?;
    
    Ok(())
}

pub fn transfer_wrapped_sol<'a>(
    signer_info: &'a AccountInfo<'a>,
    program_signer_info: &'a AccountInfo<'a>,
    amount: u64,
    user_wrapped_sol_info: &'a AccountInfo<'a>,
    destination_wrapped_sol_info: &'a AccountInfo<'a>,
    mint_info: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let bump = validate_program_signer(program_signer_info)?;
    
    let transfer_ix = fogo_sessions_sdk::token::instruction::transfer_checked(
        token_program.key,
        user_wrapped_sol_info.key,
        mint_info.key,
        destination_wrapped_sol_info.key,
        signer_info.key,
        Some(program_signer_info.key),
        amount,
        9,
    )?;
    
    solana_invoke_signed(
        &transfer_ix,
        &[
            user_wrapped_sol_info.clone(),
            mint_info.clone(),
            destination_wrapped_sol_info.clone(),
            signer_info.clone(),
            token_program.clone(),
            program_signer_info.clone(),
        ],
        &[&[PROGRAM_SIGNER_SEED, &[bump]]],
    )?;
    
    Ok(())
}

pub fn transfer_token_with_program_signer<'a>(
    token_program: &'a AccountInfo<'a>,
    sender_info: &'a AccountInfo<'a>,
    mint_info: &'a AccountInfo<'a>,
    destination_info: &'a AccountInfo<'a>,
    signer_info: &'a AccountInfo<'a>,
    program_signer_info: &'a AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    let bump = validate_program_signer(program_signer_info)?;
    
    let transfer_ix = fogo_sessions_sdk::token::instruction::transfer_checked(
        token_program.key,
        sender_info.key,
        mint_info.key,
        destination_info.key,
        signer_info.key,
        Some(program_signer_info.key),
        amount,
        11,
    )?;
    
    solana_invoke_signed(
        &transfer_ix,
        &[
            sender_info.clone(),
            mint_info.clone(),
            destination_info.clone(),
            signer_info.clone(),
            token_program.clone(),
            program_signer_info.clone(),
        ],
        &[&[PROGRAM_SIGNER_SEED, &[bump]]],
    )?;
    
    Ok(())
}
