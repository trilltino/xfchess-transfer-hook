// Lets a program move hooked Token-2022 tokens by CPI by forwarding the resolved extra accounts, hook program and list.

use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{StateWithExtensions, transfer_hook},
    instruction,
    state::Mint,
};
use spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi;

#[allow(clippy::too_many_arguments)]
/// Performs a checked Token-2022 transfer and resolves the mint's hook accounts.
///
/// `additional_accounts` must contain the hook program, metadata-list account, and accounts
/// needed to resolve the metadata entries. Supply `signer_seeds` when `authority` is a PDA
/// controlled by the calling program.
pub fn transfer_checked<'info>(
    token_program_id: &Pubkey,
    source_token_account: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    destination_token_account: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    additional_accounts: &[AccountInfo<'info>],
    amount: u64,
    decimals: u8,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut instruction = instruction::transfer_checked(
        token_program_id,
        source_token_account.key,
        mint.key,
        destination_token_account.key,
        authority.key,
        &[],
        amount,
        decimals,
    )?;
    let mut account_infos = vec![
        source_token_account.clone(),
        mint.clone(),
        destination_token_account.clone(),
        authority.clone(),
    ];
    let hook_program_id = {
        let mint_data = mint.try_borrow_data()?;
        transfer_hook::get_program_id(&StateWithExtensions::<Mint>::unpack(&mint_data)?)
    };
    if let Some(hook_program_id) = hook_program_id {
        add_extra_accounts_for_execute_cpi(
            &mut instruction,
            &mut account_infos,
            &hook_program_id,
            source_token_account.clone(),
            mint.clone(),
            destination_token_account.clone(),
            authority.clone(),
            amount,
            additional_accounts,
        )?;
    }
    invoke_signed(&instruction, &account_infos, signer_seeds)?;
    Ok(())
}
