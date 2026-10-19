use crate::client::DockerClient;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, image_pull::*};
use futures_util::{StreamExt, future::BoxFuture};
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
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}
