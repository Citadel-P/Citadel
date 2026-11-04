use crate::{
    CheckState, Release, ReleaseChannel, ReleaseCheckRepository, ReleaseNotice, ReleaseSource,
    UpdateError, available_update, development_update,
};
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy)]
pub struct ReleaseCheckPolicy {
    pub initialized: bool,
    pub checks_allowed: bool,
}

pub trait UpdateChangeNotifier: Send + Sync {
    fn changed(&self);
}

#[derive(Default)]
pub struct NoopUpdateChangeNotifier;

impl UpdateChangeNotifier for NoopUpdateChangeNotifier {
    fn changed(&self) {}
}

#[derive(Clone)]
pub struct ReleaseCheckService {
    current_version: String,
    source: Arc<dyn ReleaseSource>,
    repository: Arc<dyn ReleaseCheckRepository>,
    available: Arc<RwLock<Option<ReleaseNotice>>>,
    check_lock: Arc<tokio::sync::Mutex<()>>,
    notifier: Arc<dyn UpdateChangeNotifier>,
    agents: Option<crate::AgentUpdateService>,
    installer: Option<crate::UpdateInstallService>,
}

impl ReleaseCheckService {
    pub fn new(
        current_version: String,
        source: Arc<dyn ReleaseSource>,
        repository: Arc<dyn ReleaseCheckRepository>,
    ) -> Self {
        Self {
            current_version,
            source,
            repository,
            available: Default::default(),
            check_lock: Default::default(),
            notifier: Arc::new(NoopUpdateChangeNotifier),
            agents: None,
            installer: None,
        }
    }

    pub fn with_installer(mut self, installer: Option<crate::UpdateInstallService>) -> Self {
        self.installer = installer;
        self
    }

    pub async fn resolve_installed_alert(&self) -> Result<(), UpdateError> {
        self.repository
            .observe_core_availability(&self.current_version, None)
            .await
    }

    pub fn with_agents(mut self, agents: crate::AgentUpdateService) -> Self {
        self.agents = Some(agents);
        self
    }

    pub fn with_notifier(mut self, notifier: Arc<dyn UpdateChangeNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    pub fn installed_version(&self) -> &str {
        &self.current_version
    }

    pub fn channel(&self) -> ReleaseChannel {
        ReleaseChannel::for_version(&self.current_version)
    }

    pub fn describe(&self, checks: CheckState) -> crate::ReleaseCheckStatus {
        crate::ReleaseCheckStatus {
            installed_version: self.current_version.clone(),
            channel: self.channel(),
            checks,
        }
    }

    pub fn available(&self) -> Option<ReleaseNotice> {
        self.available
            .read()
            .expect("release cache poisoned")
            .clone()
    }

    fn publish(&self, available: Option<ReleaseNotice>, persisted: bool) {
        let changed = {
            let mut current = self.available.write().expect("release cache poisoned");
            let changed = *current != available;
            *current = available;
            changed
        };
        // No await between updating the cache and notifying readers. A committed
        // check changes timestamps/errors even when its selected release is unchanged.
        if changed || persisted {
            self.notifier.changed();
        }
    }

    pub fn clear_notice(&self) {
        self.publish(None, false);
    }

    pub async fn restore_notice(&self) -> Result<(), UpdateError> {
        let _guard = self.check_lock.lock().await;
        let state = self.status().await?;
        self.publish(state.available, false);
        Ok(())
    }

    pub async fn status(&self) -> Result<CheckState, UpdateError> {
        let mut state = self.repository.checks().await?;
        if let Some(notice) = state.available.take() {
            let prerelease = semver::Version::parse(&notice.version)
                .is_ok_and(|version| !version.pre.is_empty());
            state.available = available_update(
                &self.current_version,
                Release {
                    tag_name: format!("v{}", notice.version),
                    draft: false,
                    prerelease,
                },
            )
            .ok()
            .flatten();
        }
        Ok(state)
    }

    /// Cancellation applies to admission and fetching. Once persistence starts,
    /// the caller must await completion, including cache publication, rather than
    /// dropping this future in an outer cancellation select.
    pub async fn check_now(
        &self,
        policy: ReleaseCheckPolicy,
        cancellation: &CancellationToken,
    ) -> Result<CheckState, UpdateError> {
        self.check(policy, None, cancellation).await
    }

    /// Recheck the schedule under the same lock as manual checks. A manual check
    /// completing while this attempt waits must postpone the scheduled fetch.
    pub async fn check_scheduled(
        &self,
        policy: ReleaseCheckPolicy,
        interval: chrono::Duration,
        cancellation: &CancellationToken,
    ) -> Result<CheckState, UpdateError> {
        self.check(policy, Some(interval), cancellation).await
    }

    async fn check(
        &self,
        policy: ReleaseCheckPolicy,
        interval: Option<chrono::Duration>,
        cancellation: &CancellationToken,
    ) -> Result<CheckState, UpdateError> {
        if !policy.initialized || !policy.checks_allowed {
            return Err(UpdateError::Unavailable(
                "Release checks are disabled by server configuration or setup is incomplete."
                    .into(),
            ));
        }
        let _guard = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
            guard = self.check_lock.lock() => guard,
        };
        if let Some(interval) = interval {
            let state = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                result = self.status() => result?,
            };
            if state
                .last_attempt
                .is_some_and(|last| last + interval > chrono::Utc::now())
            {
                return Ok(state);
            }
        }
        let fetched = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
            releases = self.source.releases(self.channel()) => releases,
        };
        let result = fetched.and_then(|releases| match self.channel() {
            ReleaseChannel::Development => development_update(&self.current_version, releases),
            ReleaseChannel::Stable => available_update(
                &self.current_version,
                releases.into_iter().next().ok_or_else(|| {
                    UpdateError::Unavailable(
                        "The release source returned no stable release.".into(),
                    )
                })?,
            ),
        });
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        self.repository
            .record_check(
                result
                    .as_ref()
                    .map(|notice| notice.as_ref())
                    .map_err(|_| "Release check failed"),
                chrono::Utc::now(),
            )
            .await?;
        let checked = result.is_ok();
        match result {
            Ok(notice) => self.publish(notice, true),
            // The repository retains the last successful result on failure.
            Err(_) => self.publish(self.available(), true),
        }
        self.resolve_installed_alert().await?;
        if checked
            && let Some(notice) = self.available()
            && let Some(installer) = &self.installer
        {
            // Qualification performs no claim, approval or installation. Failed or
            // unsupported inspection must neither advertise nor clear an old alert.
            if installer
                .qualify_availability(&notice.version, cancellation)
                .await
                .is_ok()
            {
                self.repository
                    .observe_core_availability(&self.current_version, Some(&notice.version))
                    .await?;
            }
        }
        if let Some(agents) = &self.agents {
            agents.check(cancellation).await?;
        }
        self.status().await
    }
}
