// Reads a Token-2022 mint's TransferHook extension and runs the setup check that the hook authority signs.

use anchor_lang::prelude::*;
use anchor_spl::token_2022::{
    ID as TOKEN_2022_PROGRAM_ID,
    spl_token_2022::{
        extension::{
            BaseStateWithExtensions, ExtensionType, StateWithExtensions,
            transfer_hook::{TransferHook, get_program_id},
        },
        state::Mint,
    },
};

use crate::error::TransferHookError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Relevant configuration and base fields parsed from a Token-2022 hook mint.
pub struct HookMint {
    /// Program selected by the mint's Transfer Hook extension, if configured.
    pub hook_program_id: Option<Pubkey>,
    /// Authority that may update the Transfer Hook extension, if not revoked.
    pub transfer_hook_authority: Option<Pubkey>,
    /// Minting authority, if not revoked.
    pub mint_authority: Option<Pubkey>,
    /// Current token supply.
    pub supply: u64,
    /// Decimal places used for display and checked transfers.
    pub decimals: u8,
}

impl HookMint {
    /// Reads the mint's Transfer Hook extension and base mint fields.
    pub fn read(mint: &AccountInfo) -> Result<Self> {
        if mint.owner != &TOKEN_2022_PROGRAM_ID {
            return err!(TransferHookError::MintNotToken2022);
        }
        let mint_data = mint.try_borrow_data()?;
        let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;
        let transfer_hook_extension = mint_state
            .get_extension::<TransferHook>()
            .map_err(|_| TransferHookError::MintHookExtensionMissing)?;
        Ok(Self {
            hook_program_id: get_program_id(&mint_state),
            transfer_hook_authority: Option::<Pubkey>::from(transfer_hook_extension.authority),
            mint_authority: Option::<Pubkey>::from(mint_state.base.mint_authority),
            supply: mint_state.base.supply,
            decimals: mint_state.base.decimals,
        })
    }

    /// Fails unless this mint selects `program_id` as its hook program.
    pub fn require_hook_program(&self, program_id: &Pubkey) -> Result<()> {
        require!(
            self.hook_program_id == Some(*program_id),
            TransferHookError::IncorrectHookProgram
        );
        Ok(())
    }

    /// Fails unless the hook authority account signs and matches the mint configuration.
    fn require_hook_authority(&self, signer: &AccountInfo) -> Result<()> {
        if !signer.is_signer {
            return Err(ProgramError::MissingRequiredSignature.into());
        }
        let expected_authority = self
            .transfer_hook_authority
            .ok_or(TransferHookError::HookAuthorityUnavailable)?;
        require_keys_eq!(
            expected_authority,
            *signer.key,
            TransferHookError::IncorrectHookAuthority
        );
        Ok(())
    }

    /// Fails unless the minting authority has been revoked.
    pub fn require_mint_authority_revoked(&self) -> Result<()> {
        require!(
            self.mint_authority.is_none(),
            TransferHookError::MintAuthorityNotRevoked
        );
        Ok(())
    }
}

/// Reads a mint and verifies that the current program and signer match its hook configuration.
pub fn validate_hook_setup(
    program_id: &Pubkey,
    mint: &AccountInfo,
    transfer_hook_authority: &AccountInfo,
) -> Result<HookMint> {
    let transfer_hook_mint = HookMint::read(mint)?;
    transfer_hook_mint.require_hook_program(program_id)?;
    transfer_hook_mint.require_hook_authority(transfer_hook_authority)?;
    Ok(transfer_hook_mint)
}

/// Returns extension types from a legacy SPL Token or Token-2022 mint account.
pub fn read_mint_extensions(mint: &AccountInfo) -> Result<Vec<ExtensionType>> {
    if mint.owner != &TOKEN_2022_PROGRAM_ID && mint.owner != &anchor_spl::token::ID {
        return Err(ProgramError::IncorrectProgramId.into());
    }
    let mint_data = mint.try_borrow_data()?;
    Ok(StateWithExtensions::<Mint>::unpack(&mint_data)?.get_extension_types()?)
}

#[must_use]
/// Reports whether an extension affects confidential transfer amounts or balances.
pub fn is_confidential_extension(extension: ExtensionType) -> bool {
    matches!(
        extension,
        ExtensionType::ConfidentialTransferMint
            | ExtensionType::ConfidentialTransferFeeConfig
            | ExtensionType::ConfidentialMintBurn
    )
}

/// Reports whether a mint has any extension classified as confidential by this kit.
pub fn has_confidential_extension(mint: &AccountInfo) -> Result<bool> {
    Ok(read_mint_extensions(mint)?
        .into_iter()
        .any(is_confidential_extension))
}
