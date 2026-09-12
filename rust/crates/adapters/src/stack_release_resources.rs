//! Capture immutable resource identities before acknowledging a healthy Swarm apply.
use citadel_platforms::{PlatformInventoryPort, RuntimeSwarmResourceMount, RuntimeSwarmService};
use citadel_stacks::{StackError, StackOperationClaim};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Mount {
    service_name: String,
    target_name: String,
}
#[derive(Debug, PartialEq, Eq)]
struct Resource {
    kind: &'static str,
    id: String,
    name: String,
    compose_name: String,
    mounts: BTreeSet<Mount>,
}

pub(super) async fn capture(
    pool: &PgPool,
    claim: &StackOperationClaim,
    runtime: &dyn PlatformInventoryPort,
    cancel: &CancellationToken,
) -> Result<(), StackError> {
    let (info, services) = tokio::try_join!(
        runtime.get_info(cancel),
        runtime.list_swarm_services(cancel)
    )
    .map_err(runtime_error)?;
    let owned: Vec<_> = services
        .iter()
        .filter(|s| {
            s.labels
                .get("com.citadel.managed")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"))
                && s.labels
                    .get("com.citadel.stack-id")
                    .is_some_and(|v| v == &claim.stack_id.to_string())
                && s.labels
                    .get("com.citadel.release-id")
                    .is_some_and(|v| v == &claim.release_id.to_string())
                && s.labels
                    .get("com.docker.stack.namespace")
                    .is_some_and(|v| v == &claim.project_name)
        })
        .collect();
    if owned.is_empty()
        || owned.iter().any(|s| {
            s.labels
                .get("com.citadel.stack-service-count")
                .and_then(|v| v.parse::<usize>().ok())
                != Some(owned.len())
        })
    {
        return Err(runtime_error(
            "The complete set of owned release Services could not be observed.",
        ));
    }
    let secrets = if owned.iter().any(|s| !s.secret_ids.is_empty()) {
        runtime
            .list_swarm_secrets(cancel)
            .await
            .map_err(runtime_error)?
    } else {
        vec![]
    };
    let configs = if owned.iter().any(|s| !s.config_ids.is_empty()) {
        runtime
            .list_swarm_configs(cancel)
            .await
            .map_err(runtime_error)?
    } else {
        vec![]
    };
    let mut resources = collect(
        "Secret",
        &claim.project_name,
        &owned,
        secrets.iter().map(|r| (&r.id, &r.name, &r.labels)),
        |s| (&s.secret_ids, &s.secret_mounts),
    )?;
    resources.extend(collect(
        "Config",
        &claim.project_name,
        &owned,
        configs.iter().map(|r| (&r.id, &r.name, &r.labels)),
        |s| (&s.config_ids, &s.config_mounts),
    )?);
    let mut tx = pool.begin().await.map_err(runtime_error)?;
    let row = sqlx::query("SELECT p.clusterid,p.platformdescriptor FROM platforms p JOIN stacks s ON s.id=$2 JOIN stackreleases r ON r.id=$3 AND r.stackid=s.id AND r.platformid=p.id WHERE p.id=$1 AND s.currentstackreleaseid=r.id AND s.rowversion=$4 AND s.controlstate='Processing' AND r.status='Applying' FOR UPDATE OF p,s,r")
        .bind(claim.platform_id).bind(claim.stack_id).bind(claim.release_id).bind(claim.row_version).fetch_optional(&mut *tx).await.map_err(runtime_error)?
        .ok_or_else(|| StackError::Conflict("The Stack operation changed before immutable resources could be recorded.".into()))?;
    if !citadel_platforms::swarm_mutations::manager_matches(
        &info,
        row.try_get::<Option<String>, _>("clusterid")
            .map_err(runtime_error)?
            .as_deref(),
        &row.try_get("platformdescriptor").map_err(runtime_error)?,
    ) {
        return Err(runtime_error(
            "The connected Swarm manager identity changed.",
        ));
    }
    sqlx::query("DELETE FROM stackreleaseswarmresources WHERE stackreleaseid=$1")
        .bind(claim.release_id)
        .execute(&mut *tx)
        .await
        .map_err(runtime_error)?;
    for resource in resources {
        sqlx::query("INSERT INTO stackreleaseswarmresources(id,composeresourcename,dockerresourceid,dockerresourcename,kind,mounts,platformid,stackreleaseid) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(uuid::Uuid::now_v7()).bind(resource.compose_name).bind(resource.id).bind(resource.name).bind(resource.kind)
            .bind(serde_json::to_value(resource.mounts).map_err(runtime_error)?).bind(claim.platform_id).bind(claim.release_id)
            .execute(&mut *tx).await.map_err(runtime_error)?;
    }
    tx.commit().await.map_err(runtime_error)
}

fn collect<'a>(
    kind: &'static str,
    namespace: &str,
    services: &[&RuntimeSwarmService],
    observed: impl Iterator<Item = (&'a String, &'a String, &'a BTreeMap<String, String>)>,
    references: impl Fn(&RuntimeSwarmService) -> (&Vec<String>, &Vec<RuntimeSwarmResourceMount>),
) -> Result<Vec<Resource>, StackError> {
    let mut by_id = BTreeMap::new();
    for (id, name, labels) in observed {
        if by_id.insert(id, (name, labels)).is_some() {
            return Err(runtime_error(format!(
                "Duplicate Swarm {kind} identity '{id}'."
            )));
        }
    }
    let required: BTreeSet<_> = services.iter().flat_map(|s| &references(s).0[..]).collect();
    let mut result = vec![];
    for id in required {
        let (name, labels) = by_id.get(id).ok_or_else(|| {
            runtime_error(format!(
                "Referenced Swarm {kind} '{id}' could not be inspected."
            ))
        })?;
        // External materials must exist, but are not owned or retained by this Stack.
        if labels.get("com.docker.stack.namespace").map(String::as_str) != Some(namespace) {
            continue;
        }
        let mut mounts = BTreeSet::new();
        for service in services {
            for reference in references(service).1 {
                if &reference.resource_id == id && !reference.target_name.trim().is_empty() {
                    mounts.insert(Mount {
                        service_name: strip_namespace(&service.name, namespace),
                        target_name: reference.target_name.clone(),
                    });
                }
            }
        }
        if mounts.is_empty() {
            return Err(runtime_error(format!(
                "Referenced Swarm {kind} '{id}' has no observable Service mount."
            )));
        }
        result.push(Resource {
            kind,
            id: id.clone(),
            name: (*name).clone(),
            compose_name: strip_namespace(name, namespace),
            mounts,
        });
    }
    Ok(result)
}
fn strip_namespace(name: &str, namespace: &str) -> String {
    name.strip_prefix(&format!("{namespace}_"))
        .unwrap_or(name)
        .to_owned()
}
fn runtime_error(error: impl std::fmt::Display) -> StackError {
    StackError::Runtime(format!(
        "Immutable Swarm release resources could not be recorded: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn immutable_capture_preserves_ids_names_and_deduplicated_service_mounts() {
        let labels = BTreeMap::from([("com.docker.stack.namespace".into(), "demo".into())]);
        let id = "immutable-config-id".to_owned();
        let name = "demo_settings".to_owned();
        let mount = RuntimeSwarmResourceMount {
            resource_id: id.clone(),
            target_name: "/etc/settings".into(),
        };
        let mut api = RuntimeSwarmService {
            name: "demo_api".into(),
            config_ids: vec![id.clone()],
            config_mounts: vec![mount.clone(), mount.clone()],
            ..Default::default()
        };
        let worker = RuntimeSwarmService {
            name: "demo_worker".into(),
            ..api.clone()
        };
        fn refs(s: &RuntimeSwarmService) -> (&Vec<String>, &Vec<RuntimeSwarmResourceMount>) {
            (&s.config_ids, &s.config_mounts)
        }
        let items = collect(
            "Config",
            "demo",
            &[&api, &worker],
            [(&id, &name, &labels)].into_iter(),
            refs,
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, id);
        assert_eq!(items[0].compose_name, "settings");
        assert_eq!(
            serde_json::to_value(&items[0].mounts).unwrap(),
            serde_json::json!([
                {"ServiceName":"api","TargetName":"/etc/settings"},
                {"ServiceName":"worker","TargetName":"/etc/settings"}
            ])
        );
        assert!(collect("Config", "demo", &[&api], std::iter::empty(), refs).is_err());
        api.config_mounts.clear();
        assert!(
            collect(
                "Config",
                "demo",
                &[&api],
                [(&id, &name, &labels)].into_iter(),
                refs
            )
            .is_err()
        );
        let external = BTreeMap::new();
        assert!(
            collect(
                "Config",
                "demo",
                &[&api],
                [(&id, &name, &external)].into_iter(),
                refs
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            collect(
                "Config",
                "demo",
                &[&api],
                [(&id, &name, &labels), (&id, &name, &labels)].into_iter(),
                refs
            )
            .is_err()
        );
    }
    #[test]
    fn agent_inventory_keeps_secret_and_config_mount_targets() {
        use citadel_contracts::citadel::swarm::v1::*;
        let service = crate::agent::map_swarm_service(SwarmServiceMessage {
            definition: Some(SwarmServiceMutationSpecMessage {
                secrets: vec![SwarmSecretReferenceSpecMessage {
                    id: "secret-id".into(),
                    name: "secret".into(),
                    target_name: "/run/secrets/token".into(),
                }],
                configs: vec![SwarmConfigReferenceSpecMessage {
                    id: "config-id".into(),
                    name: "config".into(),
                    target_name: "/etc/config".into(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(service.secret_mounts[0].resource_id, "secret-id");
        assert_eq!(service.secret_mounts[0].target_name, "/run/secrets/token");
        assert_eq!(service.config_mounts[0].resource_id, "config-id");
        assert_eq!(service.config_mounts[0].target_name, "/etc/config");
    }
}
