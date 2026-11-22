use citadel_backups::coverage::CoverageChangeSource;
use tokio_util::sync::CancellationToken;

pub(crate) async fn realtime_updates(
    mut source: Box<dyn CoverageChangeSource>,
    realtime: Option<crate::realtime::RealtimeHub>,
    token: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    let Some(hub) = realtime else {
        drop(source);
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.disabled("Realtime is disabled");
        }
        token.cancelled().await;
        return Ok(());
    };
    loop {
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.policy("Backup coverage changes", None);
            job.waiting("Listening for backup coverage changes", None);
        }
        let next = tokio::select! {()=token.cancelled()=>return Ok(()),next=source.next()=>next};
        let mut observation = citadel_runtime::jobs::JobHandle::iteration();
        match next {
            Ok(change) => {
                hub.publish_backup_coverage(change.as_ref());
            }
            Err(error) => {
                if let Some(i) = observation.take() {
                    i.finish(citadel_runtime::jobs::JobOutcome::Failed, None);
                }
                tracing::warn!(%error,"Backup coverage listener reconnecting");
                hub.publish_backup_coverage(None);
                tokio::select! {()=token.cancelled()=>return Ok(()),_=tokio::time::sleep(std::time::Duration::from_secs(2))=>{}}
            }
        }
        if let Some(i) = observation {
            i.finish(citadel_runtime::jobs::JobOutcome::Completed, None);
        }
    }
}
