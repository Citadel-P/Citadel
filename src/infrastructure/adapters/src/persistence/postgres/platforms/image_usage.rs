use super::images::PostgresImageMutationStore;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, image_usage::*};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

const PAGE_SIZE: usize = 50;
// Bound Compose parsing and candidate materialization per request. The cursor
// resumes scanning when sparse matches exhaust this budget.
const SCAN_LIMIT: usize = 500;

impl ImageUsageStore for PostgresImageMutationStore {
    fn summaries<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        images: &'a [citadel_platforms::ImageDetails],
    ) -> BoxFuture<'a, Result<Vec<ImageUsageSummary>, RuntimeCapabilityError>> {
        Box::pin(async move {
            use std::collections::{HashMap, HashSet};
            let ids: Vec<_> = images.iter().map(|i| i.docker_image_id.clone()).collect();
            let mut tags: HashMap<String, Vec<usize>> = HashMap::new();
            let mut locals = HashMap::new();
            let mut runtime = HashMap::new();
            for (index, image) in images.iter().enumerate() {
                runtime.insert(
                    (
                        image.docker_image_id.as_str(),
                        image.docker_node_id.as_deref(),
                    ),
                    index,
                );
                if image.docker_node_id.is_none() {
                    locals.insert(image.id, index);
                }
                for reference in image.tags.iter().chain(image.repo_digests.iter().flatten()) {
                    tags.entry(canonical_reference(reference))
                        .or_default()
                        .push(index);
                }
            }
            let rows = sqlx::query(include_str!("image_usage_summaries.sql"))
                .bind(actor.value())
                .bind(administrator)
                .bind(platform)
                .bind(&ids)
                .fetch_all(&self.0)
                .await
                .map_err(storage)?;
            let mut owners = vec![HashSet::new(); images.len()];
            let mut containers = vec![HashSet::new(); images.len()];
            for row in rows.into_iter().take(10000) {
                let relationship: String = row.get("relationship");
                let reference: Option<String> = row.get("reference");
                let mut targets = Vec::new();
                if relationship == "Container" {
                    let id: String = row.get("dockerimageid");
                    let node: Option<String> = row.get("dockernodeid");
                    if let Some(index) = runtime.get(&(id.as_str(), node.as_deref())) {
                        targets.push(*index);
                    }
                } else if relationship == "Local" {
                    if let Some(reference) = &reference {
                        if let Ok(id) = Uuid::parse_str(reference) {
                            if let Some(index) = locals.get(&id) {
                                targets.push(*index);
                            }
                        } else if let Some(index) = runtime.get(&(reference.as_str(), None)) {
                            targets.push(*index);
                        }
                    }
                } else {
                    let references: Vec<String> = if relationship == "Compose" {
                        let compose: Option<String> = row.get("compose");
                        compose
                            .and_then(|c| citadel_stacks::parse_compose(&[c]).ok())
                            .map(|m| m.services.into_iter().filter_map(|s| s.image).collect())
                            .unwrap_or_default()
                    } else {
                        let host: Option<String> = row.get("registryhost");
                        host.zip(reference)
                            .and_then(|(host, reference)| {
                                crate::connectors::routing::deployments::qualify_image_reference(
                                    &host, &reference,
                                )
                                .ok()
                            })
                            .into_iter()
                            .collect()
                    };
                    for reference in references {
                        if let Some(indices) = tags.get(&canonical_reference(&reference)) {
                            targets.extend(indices);
                        }
                    }
                }
                for index in targets {
                    if let Some(id) = row.get::<Option<String>, _>("containerid") {
                        containers[index].insert(id);
                    }
                    if let Some(id) = row.get::<Option<Uuid>, _>("ownerid") {
                        owners[index].insert((row.get::<i32, _>("kind"), id));
                    }
                }
            }
            Ok(owners
                .into_iter()
                .zip(containers)
                .map(|(owners, containers)| ImageUsageSummary {
                    containers: containers.len() as u32,
                    deployments: owners.iter().filter(|(k, _)| *k == 1).count() as u32,
                    stacks: owners.iter().filter(|(k, _)| *k == 2).count() as u32,
                    swarm_services: owners.iter().filter(|(k, _)| *k == 20).count() as u32,
                    is_partial: true,
                })
                .collect())
        })
    }

    fn deletion_snapshots<'a>(
        &'a self,
        platform: Uuid,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<ImageDeletionSnapshot>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let rows = sqlx::query(r#"SELECT i.dockerimageid,i.rowversion,i.tags::jsonb,i.controlstate,
                EXISTS(SELECT 1 FROM deployments d WHERE d.platformid=i.platformid
                    AND COALESCE(d.spec->'Image'->>'$type',d.spec->'image'->>'$type')='Local'
                    AND (COALESCE(d.spec->'Image'->>'ImageId',d.spec->'image'->>'imageId')=i.dockerimageid OR translate(replace(lower(COALESCE(d.spec->'Image'->>'ImageId',d.spec->'image'->>'imageId')),'urn:uuid:',''),'{}-','')=replace(i.id::text,'-',''))) dependency
                FROM images i WHERE i.platformid=$1 AND i.dockerimageid=ANY($2) ORDER BY i.dockerimageid"#)
                .bind(platform).bind(ids).fetch_all(&self.0).await.map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    Ok(ImageDeletionSnapshot {
                        docker_image_id: row.get("dockerimageid"),
                        row_version: row.get("rowversion"),
                        tags: serde_json::from_value(row.get("tags")).map_err(storage)?,
                        has_local_dependency: row.get("dependency"),
                        processing: row.get::<Option<String>, _>("controlstate").as_deref()
                            == Some("Processing"),
                    })
                })
                .collect()
        })
    }

    fn usage<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        location: &'a ImageLocation,
        offset: u32,
    ) -> BoxFuture<'a, Result<ImageUsage, RuntimeCapabilityError>> {
        Box::pin(async move {
            let image = sqlx::query(r#"SELECT id,tags::jsonb tags,'[]'::jsonb digests FROM images
                WHERE platformid=$1 AND dockerimageid=$2 AND $3::text IS NULL
                UNION ALL SELECT id,COALESCE(resource->'repo_tags','[]'::jsonb),COALESCE(resource->'repo_digests','[]'::jsonb) FROM swarmnodeimageprojections
                WHERE platformid=$1 AND dockerimageid=$2 AND dockernodeid=$3"#)
                .bind(platform).bind(&location.docker_image_id).bind(&location.docker_node_id)
                .fetch_optional(&self.0).await.map_err(storage)?
                .ok_or_else(|| RuntimeCapabilityError::new(RuntimeErrorKind::NotFound, "Image not found on the selected host.", false))?;
            let tags: Vec<String> =
                serde_json::from_value(image.get::<Value, _>("tags")).map_err(storage)?;
            let digests: Vec<String> =
                serde_json::from_value(image.get::<Value, _>("digests")).map_err(storage)?;
            let mut references = tags;
            references.extend(digests);
            let aliases: Vec<_> = references
                .iter()
                .map(|reference| canonical_reference(reference))
                .collect();
            let rows = sqlx::query(include_str!("image_usage.sql"))
                .bind(actor.value())
                .bind(administrator)
                .bind(platform)
                .bind(&location.docker_image_id)
                .bind(&location.docker_node_id)
                .bind(image.get::<Uuid, _>("id").to_string())
                .bind(i64::from(offset))
                .bind((SCAN_LIMIT + 1) as i64)
                .fetch_all(&self.0)
                .await
                .map_err(storage)?;
            let candidate_count = rows.len();
            let mut scanned = 0;
            let mut entries = Vec::new();
            for row in rows.into_iter().take(SCAN_LIMIT) {
                scanned += 1;
                let relationship: String = row.get("relationship");
                let mut reference: Option<String> = row.get("reference");
                if relationship == "Reference" {
                    let host: Option<String> = row.get("registryhost");
                    let Some(qualified) =
                        host.as_ref()
                            .zip(reference.as_ref())
                            .and_then(|(host, reference)| {
                                crate::connectors::routing::deployments::qualify_image_reference(
                                    host, reference,
                                )
                                .ok()
                            })
                    else {
                        continue;
                    };
                    if !aliases.contains(&canonical_reference(&qualified)) {
                        continue;
                    }
                }
                if relationship == "Compose" {
                    let compose: Option<String> = row.get("compose");
                    let Some(compose) = compose else {
                        continue;
                    };
                    // Parse only the current, already-authorized saved Web Editor text.
                    // No bindings, interpolation, Git fetch or Compose execution on this path.
                    let Ok(model) = citadel_stacks::parse_compose(&[compose]) else {
                        continue;
                    };
                    reference = model
                        .services
                        .into_iter()
                        .filter_map(|service| service.image)
                        .find(|candidate| {
                            crate::connectors::routing::deployments::qualify_image_reference(
                                "docker.io",
                                candidate,
                            )
                            .is_ok_and(|qualified| {
                                aliases.contains(&canonical_reference(&qualified))
                            })
                        });
                    if reference.is_none() {
                        continue;
                    }
                }
                let kind: Option<i32> = row.get("kind");
                entries.push(ImageUsageEntry {
                    relationship: match relationship.as_str() {
                        "Container" => ImageRelationship::Container,
                        "Local" => ImageRelationship::SavedLocalImage,
                        _ if reference.as_ref().is_some_and(|v| v.contains('@')) => {
                            ImageRelationship::SavedDigest
                        }
                        _ => ImageRelationship::SavedTag,
                    },
                    workload_kind: kind.map(|kind| match kind {
                        1 => ImageWorkloadKind::Deployment,
                        2 => ImageWorkloadKind::Stack,
                        _ => ImageWorkloadKind::SwarmService,
                    }),
                    workload_id: row.get("ownerid"),
                    workload_name: row.get("ownername"),
                    workload_status: row.get("ownerstatus"),
                    container_id: row.get("containerid"),
                    container_name: row.get("containername"),
                    container_state: row.get("state"),
                    reference,
                    observed_at: row.get("observedat"),
                    is_stale: row.get("isstale"),
                });
                if entries.len() == PAGE_SIZE {
                    break;
                }
            }
            Ok(ImageUsage { entries, next_offset: (scanned < candidate_count).then_some(offset + scanned as u32), coverage_complete: false,
                limitations: vec!["Current inventory and saved references only. Digest matching requires repository digests in inventory. Git, variable-dependent Compose, build references and historical releases may be incomplete. No known references is not a guarantee that an image is safe to delete.".into()] })
        })
    }
}

fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}

fn canonical_reference(reference: &str) -> String {
    let reference = reference.trim();
    let short = reference
        .strip_prefix("docker.io/")
        .or_else(|| reference.strip_prefix("index.docker.io/"))
        .or_else(|| reference.strip_prefix("registry-1.docker.io/"))
        .unwrap_or(reference);
    let first = short.split('/').next().unwrap_or_default();
    let private =
        short.contains('/') && (first.contains('.') || first.contains(':') || first == "localhost");
    let mut result = if private {
        short.to_owned()
    } else if short.contains('/') {
        format!("docker.io/{short}")
    } else {
        format!("docker.io/library/{short}")
    };
    if !result.contains('@') && !result.rsplit('/').next().unwrap_or_default().contains(':') {
        result.push_str(":latest");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::canonical_reference;
    #[test]
    fn reference_matching_preserves_tags_digests_and_registry_identity() {
        for name in [
            "nginx",
            "library/nginx:latest",
            "docker.io/nginx",
            "index.docker.io/library/nginx:latest",
        ] {
            assert_eq!(canonical_reference(name), "docker.io/library/nginx:latest");
        }
        assert_eq!(
            canonical_reference("localhost:5000/team/app:ReleaseA"),
            "localhost:5000/team/app:ReleaseA"
        );
        assert_ne!(
            canonical_reference("private.example/nginx"),
            canonical_reference("nginx")
        );
        assert_ne!(
            canonical_reference("app:ReleaseA"),
            canonical_reference("app:releasea")
        );
        assert_eq!(
            canonical_reference("nginx@sha256:abcd"),
            "docker.io/library/nginx@sha256:abcd"
        );
    }
}
