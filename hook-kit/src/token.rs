// Reads a token account during Execute: the post-transfer balance, the owner and the transferring flag.

use anchor_lang::prelude::*;
use anchor_spl::token_2022::{
    ID as TOKEN_2022_PROGRAM_ID,
    spl_token_2022::{
        extension::{
            BaseStateWithExtensions, StateWithExtensions, immutable_owner::ImmutableOwner,
            transfer_hook::TransferHookAccount,
        },
        state::Account as TokenAccountState,
    },
};

use crate::error::TransferHookError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Small snapshot of token-account fields used by transfer policies.
pub struct TokenView {
    /// Mint that owns this token account's balance.
    pub mint: Pubkey,
    /// Wallet or program-derived address that owns the account.
    pub owner: Pubkey,
    /// Token amount currently recorded in the account.
    pub amount: u64,
    /// Whether Token-2022 marked the account as participating in a hook transfer.
    pub transferring: bool,
}

/// Checks whether a Token-2022 token account has the `ImmutableOwner` extension.
pub fn has_immutable_owner(account: &AccountInfo) -> Result<bool> {
    if account.owner != &TOKEN_2022_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId.into());
    }
    let data = account.try_borrow_data()?;
    let state = StateWithExtensions::<TokenAccountState>::unpack(&data)?;
    Ok(state.get_extension::<ImmutableOwner>().is_ok())
}

/// Fails with `DestinationOwnerNotImmutable` unless a policy keyed to the destination's owner can
/// rely on that owner staying put.
///
/// A hook that looks up eligibility by `destination.owner` must call this: otherwise the owner
/// could be reassigned after the check and carry the tokens to a wallet that was never approved.
#[inline]
pub fn require_immutable_destination_owner(destination_token_account: &AccountInfo) -> Result<()> {
    require!(
        has_immutable_owner(destination_token_account)?,
        TransferHookError::DestinationOwnerNotImmutable
    );
    Ok(())
}

/// Reads the mint, owner, balance and transfer flag from a Token-2022 token account.
pub fn read_token_view(account: &AccountInfo) -> Result<TokenView> {
    if account.owner != &TOKEN_2022_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId.into());
    }
    let data = account.try_borrow_data()?;
    let state = StateWithExtensions::<TokenAccountState>::unpack(&data)?;
    let transferring = state
        .get_extension::<TransferHookAccount>()
        .map(|extension| bool::from(extension.transferring))
        .unwrap_or(false);
    Ok(TokenView {
        mint: state.base.mint,
        owner: state.base.owner,
        amount: state.base.amount,
        transferring,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_spl::token_2022::spl_token_2022::{
        extension::{BaseStateWithExtensionsMut, ExtensionType, StateWithExtensionsMut},
        state::AccountState,
    };

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    struct Fixture {
        key: Pubkey,
        owner: Pubkey,
        lamports: u64,
        data: Vec<u8>,
    }

    impl Fixture {
        fn token_account(
            program: Pubkey,
            extensions: &[ExtensionType],
            transferring: bool,
        ) -> TestResult<Self> {
            let mut data =
                vec![0; ExtensionType::try_calculate_account_len::<TokenAccountState>(extensions)?];
            let mut state =
                StateWithExtensionsMut::<TokenAccountState>::unpack_uninitialized(&mut data)?;
            for extension in extensions {
                match extension {
                    ExtensionType::ImmutableOwner => {
                        state.init_extension::<ImmutableOwner>(true)?;
                    }
                    ExtensionType::TransferHookAccount => {
                        state
                            .init_extension::<TransferHookAccount>(true)?
                            .transferring = transferring.into();
                    }
                    _ => return Err("unsupported fixture extension".into()),
                }
            }
            state.base = TokenAccountState {
                mint: Pubkey::new_from_array([3; 32]),
                owner: Pubkey::new_from_array([4; 32]),
                amount: 55,
                state: AccountState::Initialized,
                ..TokenAccountState::default()
            };
            state.pack_base();
            state.init_account_type()?;
            Ok(Self {
                key: Pubkey::new_unique(),
                owner: program,
                lamports: 1,
                data,
            })
        }

        fn info(&mut self) -> AccountInfo<'_> {
            AccountInfo::new(
                &self.key,
                false,
                false,
                &mut self.lamports,
                &mut self.data,
                &self.owner,
                false,
            )
        }
    }

    #[test]
    fn view_reports_fields_and_the_transferring_flag() -> TestResult {
        for transferring in [true, false] {
            let mut fixture = Fixture::token_account(
                TOKEN_2022_PROGRAM_ID,
                &[ExtensionType::TransferHookAccount],
                transferring,
            )?;
            assert_eq!(
                read_token_view(&fixture.info())?,
                TokenView {
                    mint: Pubkey::new_from_array([3; 32]),
                    owner: Pubkey::new_from_array([4; 32]),
                    amount: 55,
                    transferring,
                }
            );
        }
        Ok(())
    }

    #[test]
    fn account_without_the_hook_extension_is_not_transferring() -> TestResult {
        let mut fixture = Fixture::token_account(TOKEN_2022_PROGRAM_ID, &[], false)?;
        assert!(!read_token_view(&fixture.info())?.transferring);
        Ok(())
    }

    #[test]
    fn accounts_outside_token_2022_are_refused() -> TestResult {
        let mut fixture = Fixture::token_account(Pubkey::new_unique(), &[], false)?;
        let error: ProgramError = read_token_view(&fixture.info())
            .err()
            .ok_or("expected an error")?
            .into();
        assert_eq!(error, ProgramError::IncorrectProgramId);
        assert!(has_immutable_owner(&fixture.info()).is_err());
        assert!(require_immutable_destination_owner(&fixture.info()).is_err());
        Ok(())
    }

    #[test]
    fn mutable_destination_owner_is_refused_with_the_shared_code() -> TestResult {
        let mut mutable = Fixture::token_account(TOKEN_2022_PROGRAM_ID, &[], false)?;
        assert!(!has_immutable_owner(&mutable.info())?);
        let error: ProgramError = require_immutable_destination_owner(&mutable.info())
            .err()
            .ok_or("expected an error")?
            .into();
        assert_eq!(
            error,
            ProgramError::Custom(TransferHookError::DestinationOwnerNotImmutable.code())
        );

        let mut immutable = Fixture::token_account(
            TOKEN_2022_PROGRAM_ID,
            &[ExtensionType::ImmutableOwner],
            false,
        )?;
        assert!(has_immutable_owner(&immutable.info())?);
        require_immutable_destination_owner(&immutable.info())?;
        Ok(())
    }
}
