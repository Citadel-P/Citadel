use crate::connectors::agent::client::AgentClient;
use crate::connectors::docker::DockerClient;
use crate::connectors::edge::EdgeRuntime;
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind,
    images::v1::{PullImageRequest, PullImageResponse},
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, image_pull::*};
use futures_util::{StreamExt, future::BoxFuture};
use prost::Message;

use tokio_util::sync::CancellationToken;

impl ImagePullPort for DockerClient {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut stream = tokio::select! {()=cancel.cancelled()=>return Err(failure("Image pull cancelled.")), result=self.pull_image(image,auth)=>result.map_err(|_|failure("Docker rejected image pull. Check the image reference and Registry credentials."))?};
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::stream! {
                while let Some(item)=tokio::select!{()=cancel.cancelled()=>None,item=stream.next()=>item} {
                    match item {
                        Ok(item)=>yield Ok(PullImageStreamItem{id:item.id,from:item.from,stream:item.stream,status:item.status,progress_message:item.progress,error_message:item.error.or_else(||item.error_detail.map(|e|e.message)),progress:item.progress_detail.map(|p|ImagePullProgress{units:p.units,current:p.current,total:p.total,start:p.start}),..Default::default()}),
                        Err(_)=>{yield Err(failure("Docker image pull stream failed."));break;}
                    }
                }
            }) as ImagePullStream)
        })
    }
}
impl ImagePullPort for AgentClient {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut stream = self.open_image_pull(request(image, auth), cancel).await?;
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::stream! {
                while let Some(item)=tokio::select!{()=cancel.cancelled()=>None,item=stream.next()=>item} {match item {Ok(item)=>yield Ok(map(item)),Err(_)=>{yield Err(failure("Agent image pull stream failed."));break;}}}
            }) as ImagePullStream)
        })
    }
}
impl ImagePullPort for EdgeRuntime {
    fn pull_image_stream<'a>(
        &'a self,
        image: &'a str,
        auth: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImagePullStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            let mut pending = self
                .session
                .command(
                    EdgeCommandKind::ImagePullStream,
                    request(image, auth).encode_to_vec(),
                    std::time::Duration::from_secs(600),
                    true,
                )
                .map_err(crate::connectors::edge::EdgeError::runtime)?;
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::stream! {
                loop {match pending.next(&cancel).await {
                    Ok(Some(payload))=>match PullImageResponse::decode(payload.as_slice()){Ok(item)=>yield Ok(map(item)),Err(_)=>{yield Err(failure("Invalid Edge image pull response."));break;}},
                    Ok(None)=>break,
                    Err(error)=>{yield Err(error.runtime());break;}
                }}
            }) as ImagePullStream)
        })
    }
}
fn request(image: &str, auth: Option<&str>) -> PullImageRequest {
    PullImageRequest {
        from_image: image.into(),
        auth: auth.map(str::to_owned),
        ..Default::default()
    }
}
fn map(v: PullImageResponse) -> PullImageStreamItem {
    PullImageStreamItem {
        id: v.id,
        from: v.from,
        stream: v.stream,
        status: v.status,
        error_message: v.error_message,
        progress_message: v.progress_message,
        progress: v.progress.map(|p| ImagePullProgress {
            units: p.units,
            current: p.current,
            total: p.total,
            start: p.start,
        }),
        error: v.error.map(|e| ImagePullError {
            code: e.code,
            message: e.message,
        }),
        ..Default::default()
    }
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}
