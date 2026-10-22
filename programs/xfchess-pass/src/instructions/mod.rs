// Declares and re-exports XFChess Pass's instruction handlers.

pub mod execute;
pub mod initialize;
pub mod issue_eligibility;
pub mod refresh_eligibility;
pub mod revoke_eligibility;
pub mod set_paused;

pub(crate) use execute::__client_accounts_execute;
#[cfg(feature = "cpi")]
pub(crate) use execute::__cpi_client_accounts_execute;
pub use execute::{Execute, ExecuteBumps};
pub(crate) use initialize::__client_accounts_initialize;
#[cfg(feature = "cpi")]
pub(crate) use initialize::__cpi_client_accounts_initialize;
pub use initialize::{Initialize, InitializeBumps};
pub(crate) use issue_eligibility::__client_accounts_issue_eligibility;
#[cfg(feature = "cpi")]
pub(crate) use issue_eligibility::__cpi_client_accounts_issue_eligibility;
pub use issue_eligibility::{IssueEligibility, IssueEligibilityBumps};
pub(crate) use refresh_eligibility::__client_accounts_refresh_eligibility;
#[cfg(feature = "cpi")]
pub(crate) use refresh_eligibility::__cpi_client_accounts_refresh_eligibility;
pub use refresh_eligibility::{RefreshEligibility, RefreshEligibilityBumps};
pub(crate) use revoke_eligibility::__client_accounts_revoke_eligibility;
#[cfg(feature = "cpi")]
pub(crate) use revoke_eligibility::__cpi_client_accounts_revoke_eligibility;
pub use revoke_eligibility::{RevokeEligibility, RevokeEligibilityBumps};
pub(crate) use set_paused::__client_accounts_set_paused;
#[cfg(feature = "cpi")]
pub(crate) use set_paused::__cpi_client_accounts_set_paused;
pub use set_paused::{SetPaused, SetPausedBumps};
