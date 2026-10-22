//! Shared Token-2022 Transfer Hook validation, metadata and CPI helpers.

#![forbid(unsafe_code)]

/// Helpers for loading optional hook-owned state accounts and writing them back.
pub mod account;
/// Helpers for making Token-2022 transfers through CPI.
pub mod cpi;
/// Shared errors returned by hook-kit validation.
pub mod error;
/// Fixed `Execute` account types and transfer validation.
pub mod execute;
/// Builders and validators for the SPL `ExtraAccountMetaList` format.
pub mod extra_account_metas;
/// Mint parsing and Transfer Hook setup validation.
pub mod mint;
/// Token-2022 token-account parsing helpers.
pub mod token;

pub use account::{load_optional_account, store_account};
pub use cpi::transfer_checked;
pub use error::TransferHookError;
pub use execute::{ExecuteAccountInfos, FixedExecuteAccounts, TransferContext};
pub use extra_account_metas::{
    EXECUTE_DISCRIMINATOR, EXECUTE_DISCRIMINATOR_SLICE, EXTRA_ACCOUNT_METAS_SEED,
    ExtraAccountMetaSeed, TOKEN_ACCOUNT_OWNER_OFFSET, canonical_meta_list, external_pda_meta,
    fixed_meta, meta_list_address, meta_list_len, owner_pda_meta, pda_meta, write_meta_list,
};
pub use mint::{
    HookMint, has_confidential_extension, is_confidential_extension, read_mint_extensions,
    validate_hook_setup,
};
pub use token::{
    TokenView, has_immutable_owner, read_token_view, require_immutable_destination_owner,
};

#[doc(hidden)]
pub use anchor_lang;
