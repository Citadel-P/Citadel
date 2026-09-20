use super::*;

use citadel_platforms::image_pull::{ImagePullError, ImagePullPort, PullImageStreamItem};

use crate::platforms_http::dto::PullImageInput;

use futures_util::StreamExt;

static PULL_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    post,
    path = "/api/v1/images/pull",
    operation_id = "pullImage",
    tag = "Images",
    summary = "Pull a Docker image with progress",
    request_body = PullImageInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/pullImageResponse"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn pull(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<PullImageInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::image_pull::PullImageInput = input.into();
    if input.platform_id.is_nil()
        || input.registry_id.is_nil()
        || input.image_tag.trim().is_empty()
        || input.image_tag.len() > 2048
        || input.image_tag.chars().any(char::is_control)
    {
        return identity_result(
            Err(IdentityError::Validation(
                "A Platform, Registry and valid image reference are required.".into(),
            )),
            &headers,
        );
    }
    let capabilities = authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    if !capabilities.can_pull {
        return identity_result(Err(IdentityError::Forbidden), &headers);
    }
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Registry,
                input.registry_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let permit = identity_result(
        PULL_SLOTS
            .try_acquire()
            .map_err(|_| IdentityError::Conflict("Image pull capacity is busy.".into())),
        &headers,
    )?;
    let (image, auth) = match citadel_adapters::image_pull::prepare(
        &state.pool,
        input.registry_id,
        &input.image_tag,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancel = CancellationToken::new();
    let guard = cancel.clone().drop_guard();
    let (sender, receiver) = tokio::sync::mpsc::channel(16);
    let (completed, completion) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let _permit = permit;
        let result=tokio::time::timeout(std::time::Duration::from_secs(600),async {
            let runtime=runtime_for(&state,input.platform_id).await?;
            let (pull,inventory):(&dyn ImagePullPort,&dyn PlatformInventoryPort)=match &runtime {
                RuntimeRef::Local(r)=>(*r,*r),RuntimeRef::Agent(r)=>(r,r),RuntimeRef::Edge(r)=>(r,r),
            };
            let mut stream=pull.pull_image_stream(&image,auth.as_deref().map(String::as_str),&cancel).await?;
            while let Some(item)=stream.next().await {
                let item=item?;
                if item.error_message.as_ref().is_some_and(|v|!v.is_empty()) || item.error.as_ref().and_then(|e|e.message.as_ref()).is_some_and(|v|!v.is_empty()) {
                    return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Remote,"Image pull failed. Check the image reference and Registry credentials.",false));
                }
                tokio::select! {()=cancel.cancelled()=>return Err(cancelled()),result=sender.send(item)=>if result.is_err(){return Err(cancelled());}}
            }
            if cancel.is_cancelled(){return Err(cancelled());}
            let images=inventory.list_images(&cancel).await?;
            let observed=images.iter().find(|value|value.repo_tags.iter().chain(&value.repo_digests).any(|reference|same_image_reference(reference,&image))).ok_or_else(||RuntimeCapabilityError::new(RuntimeErrorKind::NotFound,"Pull completed, but Docker did not report the requested image.",false))?;
            citadel_adapters::image_pull::persist(&state.pool,input.platform_id,input.registry_id,observed).await?;
            publish_runtime_change(&state,input.platform_id,"image","create",&observed.id);
            publish_runtime_change(&state,input.platform_id,"platform","update",&input.platform_id.to_string());
            let digest=observed.repo_digests.first().and_then(|v|v.split_once('@')).map(|(_,d)|d.to_owned());
            let _=sender.send(PullImageStreamItem{docker_image_id:Some(observed.id.clone()),digest,..Default::default()}).await;
            Ok::<_,RuntimeCapabilityError>(())
        }).await;
        let error = match result {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(match e.kind {
                RuntimeErrorKind::Remote | RuntimeErrorKind::Authentication => "Image pull failed. Check the image reference and Registry credentials.".into(),
                RuntimeErrorKind::Unavailable => "Image pull could not complete or be persisted. Refresh inventory before retrying.".into(),
                _ => e.message,
            }),
            Err(_) => Some("Image pull timed out. Refresh inventory before retrying.".into()),
        };
        let terminal = error.map(|message| PullImageStreamItem {
            error_message: Some(message.clone()),
            error: Some(ImagePullError {
                code: Some(500),
                message: Some(message),
            }),
            ..Default::default()
        });
        // Completion must not wait for room in a stalled client's progress queue.
        // The body drains queued progress before delivering this final error.
        let _ = completed.send(terminal);
    });
    let mut response = no_store(progress_body(receiver, completion, guard).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

fn progress_body(
    mut receiver: tokio::sync::mpsc::Receiver<PullImageStreamItem>,
    completion: tokio::sync::oneshot::Receiver<Option<PullImageStreamItem>>,
    guard: tokio_util::sync::DropGuard,
) -> axum::body::Body {
    let stream = async_stream::stream! {
        let _guard=guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull progress serializes")));}
        if let Ok(Some(item)) = completion.await {
            if !first { yield Ok(bytes::Bytes::from_static(b",")); }
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull error serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    axum::body::Body::from_stream(stream)
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled, "Image pull cancelled.", false)
}

fn same_image_reference(left: &str, right: &str) -> bool {
    fn docker_hub_reference(value: &str) -> &str {
        let value = value
            .strip_prefix("docker.io/")
            .or_else(|| value.strip_prefix("index.docker.io/"))
            .unwrap_or(value);
        value.strip_prefix("library/").unwrap_or(value)
    }
    docker_hub_reference(left) == docker_hub_reference(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulled_image_lookup_accepts_hub_aliases_without_changing_tags_or_private_registries() {
        assert!(same_image_reference(
            "docker.io/library/nginx:ReleaseA",
            "nginx:ReleaseA"
        ));
        assert!(same_image_reference(
            "index.docker.io/team/image:Tag",
            "team/image:Tag"
        ));
        assert!(!same_image_reference("nginx:ReleaseA", "nginx:releasea"));
        assert!(!same_image_reference(
            "private.example/library/nginx:Tag",
            "nginx:Tag"
        ));
        assert!(!same_image_reference("other/nginx:Tag", "nginx:Tag"));
    }

    #[tokio::test]
    async fn terminal_error_does_not_wait_for_space_in_a_stalled_progress_queue() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (completed, completion) = tokio::sync::oneshot::channel();
        assert!(
            sender
                .try_send(PullImageStreamItem {
                    status: Some("Pulling".into()),
                    ..Default::default()
                })
                .is_ok()
        );
        assert_eq!(sender.capacity(), 0);
        assert!(
            completed
                .send(Some(PullImageStreamItem {
                    error_message: Some("Image pull timed out.".into()),
                    ..Default::default()
                }))
                .is_ok()
        );
        drop(sender);
        let cancel = CancellationToken::new();
        let bytes = axum::body::to_bytes(
            progress_body(receiver, completion, cancel.drop_guard()),
            4096,
        )
        .await
        .unwrap();
        let items: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 2);
        assert_eq!(items[0]["status"], "Pulling");
        assert_eq!(items[1]["errorMessage"], "Image pull timed out.");
    }
}
