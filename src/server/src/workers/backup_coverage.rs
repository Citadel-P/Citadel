use citadel_backups::coverage::CoverageChangeSource;
use tokio_util::sync::CancellationToken;

pub(crate) async fn realtime_updates(
    mut source: Box<dyn CoverageChangeSource>,
    realtime: Option<crate::realtime::RealtimeHub>,
    token: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    loop {
        let next = tokio::select! {()=token.cancelled()=>return Ok(()),next=source.next()=>next};
        match next {
            Ok(change) => {
                if let Some(hub) = &realtime {
                    hub.publish_backup_coverage(change.as_ref());
                }
            }
            Err(error) => {
                tracing::warn!(%error,"Backup coverage listener reconnecting");
                if let Some(hub) = &realtime {
                    hub.publish_backup_coverage(None);
                }
                tokio::select! {()=token.cancelled()=>return Ok(()),_=tokio::time::sleep(std::time::Duration::from_secs(2))=>{}}
            }
        }
    }
}
