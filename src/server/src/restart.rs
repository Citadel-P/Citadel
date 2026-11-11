//! Process restart lifecycle. Persistence owns admission; the executable owns shutdown and re-exec.
use citadel_instance::InstanceRestartService;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct RestartControl(Option<Arc<InstanceRestartService>>);

impl RestartControl {
    pub fn new(service: Option<Arc<InstanceRestartService>>) -> Self {
        Self(service)
    }

    pub async fn wait_for_request(&self) -> std::io::Result<()> {
        let Some(service) = &self.0 else {
            return std::future::pending().await;
        };
        // Wake immediately after normal commits. Reconcile once a minute to cover a
        // disconnected HTTP future during commit, when the in-process hint can be lost.
        let mut changes = service.changes();
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(60));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = timer.tick() => {},
                _ = changes.changed() => {},
            }
            match service.pending().await {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(error) => tracing::warn!(%error, "Cannot inspect Core restart request"),
            }
        }
    }

    pub async fn acknowledge(
        &self,
        http: axum_server::Handle<std::net::SocketAddr>,
        edge: axum_server::Handle<std::net::SocketAddr>,
    ) -> std::io::Result<()> {
        let Some(service) = &self.0 else {
            return Ok(());
        };
        let (http, edge) = tokio::join!(http.listening(), edge.listening());
        if http.is_none() || edge.is_none() {
            return Err(std::io::Error::other("Core listeners did not start"));
        }
        service
            .complete_startup()
            .await
            .map_err(std::io::Error::other)
    }
}

pub fn relaunch() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // Re-execute the exact loaded binary, including when its original file was replaced.
        // Environment and arguments are inherited; startup reloads persisted Instance settings.
        Err(std::process::Command::new("/proc/self/exe")
            .args(std::env::args_os().skip(1))
            .exec()
            .into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("In-app Core restart requires Linux.".into())
    }
}
