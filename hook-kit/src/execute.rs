// The per-transfer envelope every hook runs first: proves the call is a genuine Token-2022 transfer before any rule or write.

use anchor_lang::prelude::*;

use crate::{
    error::TransferHookError,
    extra_account_metas::{
        EXTRA_ACCOUNT_METAS_SEED, declared_extra_account_count, is_declared_writable,
    },
    mint::HookMint,
    token::{TokenView, read_token_view},
};

/// Fixed accounts required by a Transfer Hook `Execute` instruction.
pub struct ExecuteAccountInfos<'info> {
    /// Token account being debited.
    pub source_token_account: AccountInfo<'info>,
    /// Token-2022 mint for the transfer.
    pub mint: AccountInfo<'info>,
    /// Token account receiving the tokens.
    pub destination_token_account: AccountInfo<'info>,
    /// Owner or delegate authorized for the transfer; it need not sign this hook call.
    pub owner_or_delegate: AccountInfo<'info>,
    /// PDA holding this hook's canonical extra-account metadata.
    pub extra_account_meta_list: AccountInfo<'info>,
    /// Bump used to derive `extra_account_meta_list`.
    pub extra_account_meta_list_bump: u8,
}

/// Converts a template's Anchor `Execute` context into the kit's fixed account bundle.
pub trait FixedExecuteAccounts<'info> {
    /// Copies the fixed accounts and metadata-list bump into `ExecuteAccountInfos`.
    fn execute_account_infos(&self, extra_account_meta_list_bump: u8)
    -> ExecuteAccountInfos<'info>;
}

/// Implements [`FixedExecuteAccounts`] for a template's conventional `Execute` account struct.
#[macro_export]
macro_rules! impl_fixed_execute_accounts {
    ($context:ident) => {
        impl<'info> $crate::FixedExecuteAccounts<'info> for $context<'info> {
            fn execute_account_infos(
                &self,
                extra_account_meta_list_bump: u8,
            ) -> $crate::ExecuteAccountInfos<'info> {
                use $crate::anchor_lang::ToAccountInfo as _;
                $crate::ExecuteAccountInfos {
                    source_token_account: self.source_token_account.to_account_info(),
                    mint: self.mint.to_account_info(),
                    destination_token_account: self.destination_token_account.to_account_info(),
                    owner_or_delegate: self.owner_or_delegate.to_account_info(),
                    extra_account_meta_list: self.extra_account_meta_list.to_account_info(),
                    extra_account_meta_list_bump,
                }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Validated values a hook's policy can use for one transfer.
pub struct TransferContext {
    /// Amount supplied to the hook's `Execute` instruction.
    pub amount: u64,
    /// Source token-account address.
    pub source_token_account: Pubkey,
    /// Transfer mint address.
    pub mint: Pubkey,
    /// Destination token-account address.
    pub destination_token_account: Pubkey,
    /// Transfer owner or delegate address.
    pub owner_or_delegate: Pubkey,
    /// Source balance observed after Token-2022 moved the tokens.
    pub source_balance_after: u64,
    /// Destination balance observed after Token-2022 moved the tokens.
    pub destination_balance_after: u64,
}

impl TransferContext {
    /// Reports whether source and destination are the same token account.
    #[must_use]
    pub fn is_self_transfer(&self) -> bool {
        self.source_token_account == self.destination_token_account
    }

    /// Returns the source balance before Token-2022 moved the tokens.
    ///
    /// A self-transfer leaves the balance unchanged. Returns `None` if undoing the debit would
    /// overflow, which a genuine transfer cannot cause.
    #[must_use]
    pub fn source_balance_before(&self) -> Option<u64> {
        if self.is_self_transfer() {
            Some(self.source_balance_after)
        } else {
            self.source_balance_after.checked_add(self.amount)
        }
    }

    /// Returns the destination balance before Token-2022 moved the tokens.
    ///
    /// A self-transfer leaves the balance unchanged. Returns `None` if undoing the credit would
    /// underflow, which a genuine transfer cannot cause.
    #[must_use]
    pub fn destination_balance_before(&self) -> Option<u64> {
        if self.is_self_transfer() {
            Some(self.destination_balance_after)
        } else {
            self.destination_balance_after.checked_sub(self.amount)
        }
    }
}

impl ExecuteAccountInfos<'_> {
    /// Validates the fixed and declared accounts before returning policy inputs.
    ///
    /// `extra_accounts` must be in the order declared by the canonical list. Unexpected
    /// `remaining_accounts` are rejected.
    pub fn validate(
        &self,
        program_id: &Pubkey,
        amount: u64,
        extra_accounts: &[&AccountInfo],
        remaining_accounts: &[AccountInfo],
        canonical_meta_list: &[u8],
    ) -> Result<TransferContext> {
        self.require_declared_account_shape(
            extra_accounts,
            remaining_accounts,
            canonical_meta_list,
        )?;
        let transfer_hook_mint = HookMint::read(&self.mint)?;
        transfer_hook_mint.require_hook_program(program_id)?;
        let (source, destination) = self.read_accounts_in_transfer()?;
        self.require_canonical_list(program_id, canonical_meta_list)?;

        Ok(TransferContext {
            amount,
            source_token_account: *self.source_token_account.key,
            mint: *self.mint.key,
            destination_token_account: *self.destination_token_account.key,
            owner_or_delegate: *self.owner_or_delegate.key,
            source_balance_after: source.amount,
            destination_balance_after: destination.amount,
        })
    }

    fn require_declared_account_shape(
        &self,
        extra_accounts: &[&AccountInfo],
        remaining_accounts: &[AccountInfo],
        canonical_meta_list: &[u8],
    ) -> Result<()> {
        let expected_extra_account_count = declared_extra_account_count(canonical_meta_list)
            .ok_or(TransferHookError::InvalidExtraAccountMetaList)?;
        if !remaining_accounts.is_empty() || extra_accounts.len() != expected_extra_account_count {
            return err!(TransferHookError::InvalidExecuteAccountCount);
        }

        let fixed_accounts_are_read_only_unsigned = [
            &self.source_token_account,
            &self.mint,
            &self.destination_token_account,
            &self.owner_or_delegate,
            &self.extra_account_meta_list,
        ]
        .iter()
        .all(|account| !account.is_writable && !account.is_signer);
        let extra_privileges_match_list =
            extra_accounts
                .iter()
                .enumerate()
                .all(|(extra_index, extra_account)| {
                    !extra_account.is_signer
                        && extra_account.is_writable
                            == is_declared_writable(canonical_meta_list, extra_index)
                });
        require!(
            fixed_accounts_are_read_only_unsigned && extra_privileges_match_list,
            TransferHookError::InvalidExecutePrivileges
        );
        Ok(())
    }

    fn read_accounts_in_transfer(&self) -> Result<(TokenView, TokenView)> {
        let source = read_token_view(&self.source_token_account)?;
        let destination = read_token_view(&self.destination_token_account)?;
        require!(
            source.mint == *self.mint.key && destination.mint == *self.mint.key,
            TransferHookError::TokenAccountMintMismatch
        );
        require!(
            source.transferring && destination.transferring,
            TransferHookError::TransferNotInProgress
        );
        Ok((source, destination))
    }

    fn require_canonical_list(
        &self,
        program_id: &Pubkey,
        canonical_meta_list: &[u8],
    ) -> Result<()> {
        let expected_address = Pubkey::create_program_address(
            &[
                EXTRA_ACCOUNT_METAS_SEED,
                self.mint.key.as_ref(),
                &[self.extra_account_meta_list_bump],
            ],
            program_id,
        )
        .map_err(|_| TransferHookError::InvalidExtraAccountMetaList)?;
        let list = &self.extra_account_meta_list;
        require!(
            list.key == &expected_address
                && list.owner == program_id
                && list.try_borrow_data()?.as_ref() == canonical_meta_list,
            TransferHookError::InvalidExtraAccountMetaList
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(source: Pubkey, destination: Pubkey, amount: u64) -> TransferContext {
        TransferContext {
            amount,
            source_token_account: source,
            mint: Pubkey::new_unique(),
            destination_token_account: destination,
            owner_or_delegate: Pubkey::new_unique(),
            source_balance_after: 70,
            destination_balance_after: 30,
        }
    }

    #[test]
    fn balances_before_undo_the_transfer() {
        let transfer = context(Pubkey::new_unique(), Pubkey::new_unique(), 30);
        assert!(!transfer.is_self_transfer());
        assert_eq!(transfer.source_balance_before(), Some(100));
        assert_eq!(transfer.destination_balance_before(), Some(0));
    }

    #[test]
    fn self_transfer_leaves_balances_unchanged() {
        let account = Pubkey::new_unique();
        let transfer = context(account, account, 30);
        assert!(transfer.is_self_transfer());
        assert_eq!(transfer.source_balance_before(), Some(70));
        assert_eq!(transfer.destination_balance_before(), Some(30));
    }

    #[test]
    fn impossible_balances_are_none_not_wrapped() {
        let (source, destination) = (Pubkey::new_unique(), Pubkey::new_unique());
        let mut transfer = context(source, destination, 31);
        assert_eq!(transfer.destination_balance_before(), None);
        transfer.source_balance_after = u64::MAX;
        assert_eq!(transfer.source_balance_before(), None);
        let account = Pubkey::new_unique();
        let mut transfer = context(account, account, u64::MAX);
        transfer.source_balance_after = u64::MAX;
        assert_eq!(transfer.source_balance_before(), Some(u64::MAX));
        assert_eq!(transfer.destination_balance_before(), Some(30));
    }
}
