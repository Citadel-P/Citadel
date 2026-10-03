use citadel_activities::ActivityEventInfo;
use citadel_licensing::{LicenseChange, LicenseState};
pub(super) fn activity(change: LicenseChange) -> ActivityEventInfo {
    match change {
        LicenseChange::Installed(state) => ActivityEventInfo::license_installed(snapshot(&state)),
        LicenseChange::Replaced(previous, current) => {
            ActivityEventInfo::license_replaced(snapshot(&previous), snapshot(&current))
        }
        LicenseChange::Removed(state) => ActivityEventInfo::license_removed(snapshot(&state)),
        LicenseChange::EnteredGracePeriod(state) => {
            ActivityEventInfo::license_entered_grace_period(snapshot(&state))
        }
        LicenseChange::Expired(state) => ActivityEventInfo::license_expired(snapshot(&state)),
        LicenseChange::ValidationFailed(fingerprint, status, code) => {
            ActivityEventInfo::license_validation_failed(fingerprint, status, code)
        }
    }
}
fn snapshot(state: &LicenseState) -> citadel_activities::LicenseActivitySnapshot {
    citadel_activities::LicenseActivitySnapshot {
        schema: state.license_schema,
        license_id: state.license_id.clone(),
        replaced_license_id: state.replaced_license_id.clone(),
        licensed_edition: state.licensed_edition.clone(),
        effective_edition: state.effective_edition.clone(),
        effective_capabilities: state.effective_capabilities.iter().copied().collect(),
        customer_id: state.customer_id.clone(),
        customer_name: state.customer_name.clone(),
        fingerprint: state.fingerprint.clone(),
        status: state.status,
        expires_at: state.expires_at,
        grace_until: state.grace_until,
    }
}
