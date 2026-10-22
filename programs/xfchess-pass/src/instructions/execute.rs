// Execute handler for XFChess Pass: the one-unit pass moves only to a wallet with a current eligibility record.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};
use hook_kit::{EXTRA_ACCOUNT_METAS_SEED, FixedExecuteAccounts};

use crate::{
    error::TournamentPassError,
    rule,
    state::{
        ELIGIBILITY_RECORD_SEED, EXTRA_ACCOUNT_META_LIST, EligibilityRecord,
        TOURNAMENT_PASS_CONFIG_SEED, TOURNAMENT_PASS_CONFIG_VERSION, TournamentPassConfig,
    },
};

#[derive(Accounts)]
pub struct Execute<'info> {
    pub source_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    pub destination_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: owner or delegate; Token-2022 authorised the transfer and does not forward the
    /// signature. Never `Signer`, never required to equal `source_token_account.owner`, and never
    /// the wallet whose eligibility is checked.
    pub owner_or_delegate: UncheckedAccount<'info>,

    /// CHECK: SPL TLV list; hook_kit::ExecuteAccountInfos::validate verifies owner and bytes.
    #[account(
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump = tournament_pass_config.extra_account_meta_list_bump,
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        seeds = [TOURNAMENT_PASS_CONFIG_SEED, mint.key().as_ref()],
        bump = tournament_pass_config.bump,
        has_one = mint @ TournamentPassError::EligibilityMismatch,
    )]
    pub tournament_pass_config: Account<'info, TournamentPassConfig>,

    /// CHECK: the receiving wallet's record, address checked by these seeds; empty and
    /// system-owned unless that wallet was registered. `load_optional_account` checks its contents.
    #[account(
        seeds = [
            ELIGIBILITY_RECORD_SEED,
            mint.key().as_ref(),
            destination_token_account.owner.as_ref(),
        ],
        bump,
    )]
    pub eligibility_record: UncheckedAccount<'info>,
}

hook_kit::impl_fixed_execute_accounts!(Execute);

pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
    let accounts = &ctx.accounts;
    let tournament_pass_config_info = accounts.tournament_pass_config.to_account_info();
    let eligibility_record_info = accounts.eligibility_record.to_account_info();
    let transfer_context = accounts
        .execute_account_infos(accounts.tournament_pass_config.extra_account_meta_list_bump)
        .validate(
            ctx.program_id,
            amount,
            &[&tournament_pass_config_info, &eligibility_record_info],
            ctx.remaining_accounts,
            &EXTRA_ACCOUNT_META_LIST,
        )?;
    require_eq!(
        accounts.tournament_pass_config.version,
        TOURNAMENT_PASS_CONFIG_VERSION,
        TournamentPassError::UnsupportedVersion
    );
    // The recipient's record is looked up by the account's owner, so the owner must not be able to
    // change afterwards and carry the pass to a wallet that was never attested.
    hook_kit::require_immutable_destination_owner(
        &accounts.destination_token_account.to_account_info(),
    )?;

    let eligibility = hook_kit::load_optional_account::<EligibilityRecord>(
        &eligibility_record_info,
        ctx.program_id,
        error!(TournamentPassError::EligibilityMismatch),
    )?;
    if let Some(eligibility) = &eligibility {
        require!(
            eligibility.mint == transfer_context.mint
                && eligibility.player == accounts.destination_token_account.owner,
            TournamentPassError::EligibilityMismatch
        );
    }
    rule::enforce_pass_transfer(
        &accounts.tournament_pass_config.policy,
        accounts.tournament_pass_config.is_paused,
        eligibility.as_ref(),
        &transfer_context,
        Clock::get()?.slot,
    )?;
    Ok(())
}
