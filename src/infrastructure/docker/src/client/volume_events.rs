//! Enrich consecutive creates once at the Agent producer before streaming them to Core.
use super::{
    DockerClient, DockerError, DockerJsonStream,
    projection::{DockerEvent, DockerVolume},
};
use citadel_platforms::jobs::{ResourceChange, RuntimeEventKind};
use futures_util::StreamExt;
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

fn created(item: &Result<DockerEvent, DockerError>) -> Option<&str> {
    let event = item.as_ref().ok()?;
    matches!(
        super::events::normalize(event),
        Some(RuntimeEventKind::Volume(ResourceChange::Observe))
    )
    .then_some(event.actor.id.as_str())
}

impl DockerClient {
    pub fn volume_event_stream(
        &self,
        mut stream: DockerJsonStream<DockerEvent>,
        cancel: CancellationToken,
    ) -> DockerJsonStream<(DockerEvent, Option<DockerVolume>)> {
        let docker = self.clone();
        Box::pin(async_stream::stream! {
            let mut carry = None;
            loop {
                let item = match carry.take() {
                    Some(item) => item,
                    None => tokio::select! {
                        () = cancel.cancelled() => break,
                        item = stream.next() => match item {
                            Some(item) => item,
                            None => break,
                        }
                    },
                };
                let Some(first) = created(&item) else {
                    yield item.map(|event| (event, None));
                    continue;
                };
                let mut names = HashSet::from([first.to_owned()]);
                let compatible = |_: &_, next: &_| created(next).is_some_and(|id| names.insert(id.to_owned()));
                let Some((items, barrier)) = citadel_runtime::event_batch::gather(item, stream.as_mut(), compatible, &cancel).await else { break };
                carry = barrier;
                let events: Vec<_> = items.into_iter().flatten().collect();
                let names = events.iter().map(|e| e.actor.id.clone()).collect::<Vec<_>>();
                let volumes = tokio::select! {
                    () = cancel.cancelled() => break,
                    result = docker.observe_created_volumes(names) => result,
                };
                let mut volumes = volumes.unwrap_or_else(|error| {
                    tracing::debug!(%error, "Volume event metadata unavailable");
                    Vec::new()
                }).into_iter();
                for event in events {
                    yield Ok((event, volumes.next().flatten()));
                }
            }
        })
    }
}
