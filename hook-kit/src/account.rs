// Loads and stores a hook's state accounts, telling an absent optional account from a malformed one, for every hook's Execute.

use anchor_lang::prelude::*;

/// Loads a hook-owned account, treating only an empty system-owned account as absent.
///
/// Returns the caller-provided error for foreign-owned or malformed accounts.
pub fn load_optional_account<T: AccountDeserialize>(
    account: &AccountInfo,
    program_id: &Pubkey,
    invalid_account: anchor_lang::error::Error,
) -> Result<Option<T>> {
    if account.owner == &System::id() && account.data_is_empty() {
        return Ok(None);
    }
    if account.owner != program_id {
        return Err(invalid_account);
    }
    T::try_deserialize(&mut &account.try_borrow_data()?[..])
        .map(Some)
        .map_err(|_| invalid_account)
}

/// Serializes `account`, discriminator included, over the start of `account_info`'s data.
///
/// Execute receives its state accounts as unchecked `AccountInfo`s, so Anchor does not write them
/// back on exit. The caller must have verified the account's address and owner.
#[inline]
pub fn store_account<T: AccountSerialize>(account: &T, account_info: &AccountInfo) -> Result<()> {
    let mut data = account_info.try_borrow_mut_data()?;
    let mut writer: &mut [u8] = &mut data;
    account.try_serialize(&mut writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    const PROGRAM_ID: Pubkey = Pubkey::new_from_array([7; 32]);

    const SAMPLE_DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    #[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq)]
    struct Sample {
        value: u64,
    }

    impl AccountSerialize for Sample {
        fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
            writer.write_all(&SAMPLE_DISCRIMINATOR)?;
            self.serialize(writer)?;
            Ok(())
        }
    }

    impl AccountDeserialize for Sample {
        fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
            let Some((discriminator, rest)) = buf.split_at_checked(SAMPLE_DISCRIMINATOR.len())
            else {
                return Err(ProgramError::InvalidAccountData.into());
            };
            if discriminator != SAMPLE_DISCRIMINATOR {
                return Err(ProgramError::InvalidAccountData.into());
            }
            Ok(Self::deserialize(&mut &*rest)?)
        }
    }

    fn account_info<'a>(
        key: &'a Pubkey,
        owner: &'a Pubkey,
        lamports: &'a mut u64,
        data: &'a mut [u8],
    ) -> AccountInfo<'a> {
        AccountInfo::new(key, false, true, lamports, data, owner, false)
    }

    fn invalid() -> anchor_lang::error::Error {
        ProgramError::InvalidAccountData.into()
    }

    fn serialized(sample: &Sample) -> TestResult<Vec<u8>> {
        let mut data = Vec::new();
        sample.try_serialize(&mut data)?;
        Ok(data)
    }

    #[test]
    fn empty_system_account_is_absent() -> TestResult {
        let (key, owner, mut lamports) = (Pubkey::new_unique(), System::id(), 0);
        let info = account_info(&key, &owner, &mut lamports, &mut []);
        assert_eq!(
            load_optional_account::<Sample>(&info, &PROGRAM_ID, invalid())?,
            None
        );
        Ok(())
    }

    #[test]
    fn foreign_owned_or_funded_system_accounts_are_refused() {
        let (key, mut lamports) = (Pubkey::new_unique(), 0);
        let foreign = Pubkey::new_unique();
        let mut data = [0u8; 16];
        let info = account_info(&key, &foreign, &mut lamports, &mut data);
        assert!(load_optional_account::<Sample>(&info, &PROGRAM_ID, invalid()).is_err());

        let system = System::id();
        let mut data = [1u8];
        let info = account_info(&key, &system, &mut lamports, &mut data);
        assert!(load_optional_account::<Sample>(&info, &PROGRAM_ID, invalid()).is_err());
    }

    #[test]
    fn malformed_program_account_is_refused() {
        let (key, mut lamports) = (Pubkey::new_unique(), 0);
        let mut data = [0u8; 16];
        let info = account_info(&key, &PROGRAM_ID, &mut lamports, &mut data);
        assert!(load_optional_account::<Sample>(&info, &PROGRAM_ID, invalid()).is_err());
    }

    #[test]
    fn stored_account_loads_back() -> TestResult {
        let sample = Sample { value: 42 };
        let (key, mut lamports) = (Pubkey::new_unique(), 0);
        let mut data = vec![0u8; serialized(&sample)?.len()];
        let info = account_info(&key, &PROGRAM_ID, &mut lamports, &mut data);
        store_account(&sample, &info)?;
        assert_eq!(
            load_optional_account::<Sample>(&info, &PROGRAM_ID, invalid())?,
            Some(sample)
        );
        Ok(())
    }

    #[test]
    fn storing_into_a_short_account_fails_without_panicking() -> TestResult {
        let sample = Sample { value: 1 };
        let (key, mut lamports) = (Pubkey::new_unique(), 0);
        let mut data = vec![0u8; serialized(&sample)?.len() - 1];
        let info = account_info(&key, &PROGRAM_ID, &mut lamports, &mut data);
        assert!(store_account(&sample, &info).is_err());
        Ok(())
    }
}
