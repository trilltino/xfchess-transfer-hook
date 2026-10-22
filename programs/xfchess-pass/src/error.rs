// XFChess Pass's error codes, with its retired codes kept reserved.

use anchor_lang::prelude::*;

#[error_code(offset = 0)]
#[derive(PartialEq, Eq)]
pub enum TournamentPassError {
    #[msg("the pass config version is not supported")]
    UnsupportedVersion = 0xD001,
    #[msg("the eligibility policy or registrar is invalid")]
    InvalidPolicy = 0xD002,
    #[msg("pass transfers are paused")]
    PassPaused = 0xD003,
    #[msg("the recipient's eligibility record is revoked")]
    EligibilityRevoked = 0xD004,
    #[msg("the recipient's eligibility record is stale or from the future")]
    EligibilityExpired = 0xD005,
    #[msg("the eligibility record is not this recipient's or this mint's")]
    EligibilityMismatch = 0xD006,
    #[msg("the recipient has no eligibility record")]
    RecipientNotEligible = 0xD007,
    #[msg("the verified rating is outside the tournament range")]
    RatingOutsideRange = 0xD008,
    #[msg("a pass transfer must move exactly one unit")]
    InvalidTransferAmount = 0xD009,
    #[msg("the destination token account must hold exactly one pass afterwards")]
    InvalidDestinationBalance = 0xD00A,
    #[msg("only the pass registrar may sign this instruction")]
    UnauthorizedRegistrar = 0xD00B,
    #[msg("a pass mint must be indivisible (zero decimals)")]
    MintNotIndivisible = 0xD00C,
    #[msg("confidential-balance mint extensions are unsupported")]
    UnsupportedMintExtension = 0xD00D,
}

pub const RETIRED_CODES: &[u32] = &[];

impl TournamentPassError {
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_unique_and_not_retired() {
        let errors = [
            TournamentPassError::UnsupportedVersion,
            TournamentPassError::InvalidPolicy,
            TournamentPassError::PassPaused,
            TournamentPassError::EligibilityRevoked,
            TournamentPassError::EligibilityExpired,
            TournamentPassError::EligibilityMismatch,
            TournamentPassError::RecipientNotEligible,
            TournamentPassError::RatingOutsideRange,
            TournamentPassError::InvalidTransferAmount,
            TournamentPassError::InvalidDestinationBalance,
            TournamentPassError::UnauthorizedRegistrar,
            TournamentPassError::MintNotIndivisible,
            TournamentPassError::UnsupportedMintExtension,
        ];
        for (index, error) in errors.into_iter().enumerate() {
            assert!((0xD001..=0xD0FF).contains(&error.code()), "{error:?}");
            assert!(!RETIRED_CODES.contains(&error.code()), "{error:?}");
            assert_eq!(error.code(), 0xD001 + index as u32, "{error:?}");
            let program_error: ProgramError = anchor_lang::error::Error::from(error).into();
            assert_eq!(program_error, ProgramError::Custom(error.code()));
        }
    }
}
