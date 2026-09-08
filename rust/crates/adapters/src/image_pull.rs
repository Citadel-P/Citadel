use crate::{agent::AgentClient, docker::DockerClient, edge::EdgeRuntime};
use citadel_contracts::citadel::{
    edge::v1::EdgeCommandKind,
    images::v1::{PullImageRequest, PullImageResponse},
};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimeImageSummary, image_pull::*,
};
use futures_util::{StreamExt, future::BoxFuture};
use prost::Message;
use serde_json::Value;
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn prepare(
    pool: &PgPool,
    id: Uuid,
    reference: &str,
) -> Result<(String, Option<zeroize::Zeroizing<String>>), RuntimeCapabilityError> {
    let row = sqlx::query("SELECT registryhost,configuration,status FROM registries WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(db)?
        .ok_or_else(|| failure("Registry not found."))?;
    if row.get::<String, _>("status") == "Disabled" {
        return Err(failure("Registry is disabled."));
    }
    let host: String = row.get("registryhost");
    let cfg: Value = row.get("configuration");
    let kind = cfg.get("$type").and_then(Value::as_str).unwrap_or_default();
    let get = |fields: &[&str]| {
        fields
            .iter()
            .find_map(|f| cfg.get(*f).and_then(Value::as_str))
    };
    let host = host
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    let mut reference = normalize_reference(reference)?;
    let default_registry = id == Uuid::from_u128(0x100);
    if !default_registry && !reference.starts_with(&format!("{host}/")) {
        let namespace = match kind {
            "DockerHub" => get(&["UserName", "userName", "username"]),
            "GitHub" => get(&["NameSpace", "nameSpace"]),
            _ => None,
        };
        reference = match namespace.filter(|v| !v.is_empty()) {
            Some(ns) => format!("{host}/{ns}/{reference}"),
            None => format!("{host}/{reference}"),
        };
    }
    let auth = crate::deployment_runtime::registry_auth(id, &host, &cfg)
        .map_err(|_| failure("Registry configuration is invalid."))?;
    Ok((reference, auth))
}

fn normalize_reference(reference: &str) -> Result<String, RuntimeCapabilityError> {
    // Tags are case-sensitive. Preserve the requested reference; Docker validates
    // repository syntax instead of silently pulling a different tag.
    let mut reference = reference.trim().to_owned();
    if reference.is_empty()
        || reference.len() > 2048
        || reference.chars().any(char::is_whitespace)
        || reference.chars().any(char::is_control)
    {
        return Err(failure("A valid image reference is required."));
    }
    if !reference.contains('@')
        && !reference
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .contains(':')
    {
        reference.push_str(":latest");
    }
    Ok(reference)
}

pub async fn persist(
    pool: &PgPool,
    platform: Uuid,
    registry: Uuid,
    image: &RuntimeImageSummary,
) -> Result<(), RuntimeCapabilityError> {
    let mut tx = pool.begin().await.map_err(db)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| failure("Platform was removed during image pull."))?;
    sqlx::query("INSERT INTO images(id,containers,createdat,dockerimageid,name,platformid,registryid,size,tags,controlstate) VALUES($1,$2,to_timestamp($3),$4,$5,$6,$7,$8,$9,'Idle') ON CONFLICT(dockerimageid,platformid) DO UPDATE SET containers=EXCLUDED.containers,name=EXCLUDED.name,registryid=EXCLUDED.registryid,size=EXCLUDED.size,tags=EXCLUDED.tags,updatedat=now(),rowversion=images.rowversion+1")
        .bind(Uuid::now_v7()).bind(i32::try_from(image.containers).unwrap_or(i32::MAX)).bind(image.created as f64).bind(&image.id).bind(image.repo_tags.first().unwrap_or(&image.id)).bind(platform).bind(registry).bind(image.size as f64).bind(serde_json::json!(image.repo_tags)).execute(&mut *tx).await.map_err(db)?;
    sqlx::query("UPDATE platforms SET imagecount=(SELECT count(*) FROM images WHERE platformid=$1) WHERE id=$1").bind(platform).execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)
}
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
                .map_err(|_| failure("Edge image pull dispatch failed."))?;
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::stream! {
                loop {match pending.next(&cancel).await {
                    Ok(Some(payload))=>match PullImageResponse::decode(payload.as_slice()){Ok(item)=>yield Ok(map(item)),Err(_)=>{yield Err(failure("Invalid Edge image pull response."));break;}},
                    Ok(None)=>break,
                    Err(_)=>{yield Err(failure("Edge image pull stream failed."));break;}
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
fn db(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, error.to_string(), false)
}

#[cfg(test)]
mod reference_tests {
    use super::*;

    #[test]
    fn pull_reference_preserves_case_sensitive_tags_and_defaults_only_untagged_images() {
        assert_eq!(
            normalize_reference(" nginx:ReleaseA ").unwrap(),
            "nginx:ReleaseA"
        );
        assert_eq!(
            normalize_reference("localhost:5000/nginx").unwrap(),
            "localhost:5000/nginx:latest"
        );
        assert_eq!(
            normalize_reference("nginx@sha256:abcdef").unwrap(),
            "nginx@sha256:abcdef"
        );
        assert!(normalize_reference("nginx bad").is_err());
    }
}
