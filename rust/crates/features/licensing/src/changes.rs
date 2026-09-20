use crate::LicenseState;
use crate::LicenseStatus;
#[derive(Debug, Clone)]
pub enum LicenseChange {
    Installed(LicenseState),
    Replaced(Box<LicenseState>, LicenseState),
    Removed(LicenseState),
    EnteredGracePeriod(LicenseState),
    Expired(LicenseState),
    ValidationFailed(Option<String>, LicenseStatus, Option<String>),
}
