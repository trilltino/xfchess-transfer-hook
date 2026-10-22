// Error codes shared by every hook (0x8001 onward).

use anchor_lang::prelude::*;

#[error_code(offset = 0)]
/// Errors from shared Transfer Hook account and mint validation.
pub enum TransferHookError {
    #[msg("the mint is not owned by Token-2022")]
    MintNotToken2022 = 0x8001,
    #[msg("the mint has no TransferHook extension")]
    MintHookExtensionMissing = 0x8002,
    #[msg("the mint's TransferHook program is not this program")]
    IncorrectHookProgram = 0x8003,
    #[msg("the signer is not the mint's TransferHook authority")]
    IncorrectHookAuthority = 0x8004,
    #[msg("the mint's TransferHook authority is revoked; nobody can set the hook up")]
    HookAuthorityUnavailable = 0x8005,
    #[msg("Execute was not called by Token-2022 during a transfer")]
    TransferNotInProgress = 0x8007,
    #[msg("the accounts do not match the ExtraAccountMetaList")]
    InvalidExecuteAccountCount = 0x8008,
    #[msg("a token account does not belong to the hooked mint")]
    TokenAccountMintMismatch = 0x8009,
    #[msg("the ExtraAccountMetaList is missing, malformed or not canonical")]
    InvalidExtraAccountMetaList = 0x800a,
    #[msg("the mint authority must be revoked first")]
    MintAuthorityNotRevoked = 0x800b,
    #[msg("an Execute account has a privilege Token-2022 never grants it")]
    InvalidExecutePrivileges = 0x800c,
    #[msg(
        "the destination token account's owner can be reassigned: it needs the ImmutableOwner extension"
    )]
    DestinationOwnerNotImmutable = 0x800d,
}

impl TransferHookError {
    /// Returns the numeric custom error code used by Solana.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}
