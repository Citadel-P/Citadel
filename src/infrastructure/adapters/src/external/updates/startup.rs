use super::{JournalDirectory, RegisteredComposeInstallation};
use citadel_updates::UpdateError;
use std::path::Path;

/// Observe registration under a short, nonblocking installation lock. Startup
/// must never erase evidence or infer successful execution from a journal flag.
/// Active or orphaned evidence keeps Core in maintenance. Completed archives
/// must carry the validated host reconciliation record.
pub fn check_startup_registration(
    path: &Path,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    let state = JournalDirectory::open(path.into())?;
    let _lock = state.existing_lock()?;
    let registration: RegisteredComposeInstallation = state.read("installation.json")?;
    registration.validate()?;
    if registration.state_directory != path
        || registration.executor_uid != rustix::process::geteuid().as_raw()
    {
        return Err(UpdateError::Conflict(
            "Update registration does not match the mounted state directory or Core user.".into(),
        ));
    }
    state.require_registration_only()?;
    state.verify_location()?;
    Ok(registration)
}
