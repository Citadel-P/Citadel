use super::*;
use axum::http::HeaderValue;
use citadel_platforms::volume_content::normalize_path;
use futures_util::StreamExt;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ContentQuery {
    path: Option<String>,
    docker_node_id: Option<String>,
}

async fn authorize(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    specific: SpecificPermission,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    authorize_platform_level(state, principal, platform, PermissionLevel::Read, headers).await?;
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::Volume,
                    platform,
                    PermissionLevel::Read,
                    Some(specific),
                )
                .await,
            headers,
        )?;
    }
    Ok(())
}

pub(super) async fn list(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, name)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Browse,
        &headers,
    )
    .await?;
    let Query(query) = identity_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    match state
        .volume_content
        .list(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(listing) => Ok(no_store(Json(listing).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

pub(super) async fn download(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, name)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Download,
        &headers,
    )
    .await?;
    let Query(query) = identity_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let guard = cancellation.clone().drop_guard();
    let download = match state
        .volume_content
        .download(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(download) => download,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let disposition = format!(
        "attachment; filename=\"download{}\"; filename*=UTF-8''{}",
        if download.directory { ".tar" } else { "" },
        urlencoding::encode(&download.filename)
    );
    let directory = download.directory;
    let filename = download.filename;
    let path = path.to_owned();
    let mut input = download.stream;
    let stream: futures_util::stream::BoxStream<'static, Result<Vec<u8>, std::io::Error>> =
        Box::pin(async_stream::try_stream! {
            let _guard = guard;
            while let Some(chunk) = input.next().await { yield chunk?; }
            let activity = citadel_adapters::activity_store::PostgresActivityStore::new(state.pool.clone());
            let details = citadel_domain::VolumeContentDownloaded { volume_name: name, path, is_directory: directory, file_name: filename };
            if !matches!(tokio::time::timeout(std::time::Duration::from_secs(5), activity.record_volume_download(principal.actor_id, platform, details)).await, Ok(Ok(()))) {
                tracing::warn!("Could not record completed Volume download activity.");
            }
        });
    let mut response = axum::response::Response::new(axum::body::Body::from_stream(stream));
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_static(if directory {
            "application/x-tar"
        } else {
            "application/octet-stream"
        }),
    );
    response.headers_mut().insert(
        "Content-Disposition",
        HeaderValue::from_str(&disposition).expect("percent-encoded download filename"),
    );
    response.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    // No guessed Content-Length: the underlying Volume is live and may change.
    Ok(no_store(response))
}
