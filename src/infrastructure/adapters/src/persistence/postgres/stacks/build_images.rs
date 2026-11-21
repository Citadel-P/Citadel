use citadel_stacks::{
    ResolvedStackBuildImageBinding, StackBuildImageBinding, StackBuildImageResolverPort, StackError,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresStackBuildImageResolver {
    pool: PgPool,
}

impl PostgresStackBuildImageResolver {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl StackBuildImageResolverPort for PostgresStackBuildImageResolver {
    fn resolve<'a>(
        &'a self,
        bindings: &'a [StackBuildImageBinding],
    ) -> BoxFuture<'a, Result<Vec<ResolvedStackBuildImageBinding>, StackError>> {
        Box::pin(async move {
            if bindings.is_empty() {
                return Ok(Vec::new());
            }
            let keys: BTreeSet<_> = bindings
                .iter()
                .map(|b| (b.build_project_id, b.resolved_build_run_id))
                .collect();
            let (projects, pinned): (Vec<_>, Vec<_>) = keys.into_iter().unzip();
            let rows = sqlx::query(r#"SELECT key.pinned, project.id AS projectid,run.id AS runid,run.imagereferences,run.imagedigest
FROM unnest($1::uuid[], $2::uuid[]) AS key(projectid,pinned)
JOIN buildprojects project ON project.id=key.projectid
LEFT JOIN LATERAL (
    SELECT candidate.id,candidate.imagereferences,candidate.imagedigest
    FROM buildruns candidate
    WHERE candidate.buildprojectid=project.id
      AND candidate.status='Succeeded'
      AND candidate.registrysnapshot <> 'null'::jsonb
      AND (key.pinned IS NULL OR candidate.id=key.pinned)
    ORDER BY candidate.completedat DESC NULLS LAST,candidate.id DESC
    LIMIT 1
) run ON TRUE
WHERE project.enabled AND project.archivedat IS NULL"#,)
                .bind(&projects).bind(&pinned).fetch_all(&self.pool).await.map_err(storage)?;
            let mut by_key = BTreeMap::new();
            for row in rows {
                let key = (
                    row.try_get::<Uuid, _>("projectid").map_err(storage)?,
                    row.try_get::<Option<Uuid>, _>("pinned").map_err(storage)?,
                );
                let run_id = row.try_get::<Option<Uuid>, _>("runid").map_err(storage)?;
                let reference = row
                    .try_get::<Option<Value>, _>("imagereferences")
                    .map_err(storage)?
                    .and_then(first_reference);
                let digest = row
                    .try_get::<Option<String>, _>("imagedigest")
                    .map_err(storage)?;
                by_key.insert(key, (run_id, reference, digest));
            }
            let mut resolved = Vec::with_capacity(bindings.len());
            for binding in bindings {
                let (run_id, reference, digest) = by_key
                    .get(&(binding.build_project_id, binding.resolved_build_run_id))
                    .ok_or_else(|| {
                        StackError::Validation(format!(
                            "Build Project for service '{}' was not found or is disabled.",
                            binding.service_name
                        ))
                    })?;
                let retained = binding
                    .resolved_image_reference
                    .as_deref()
                    .filter(|reference| !reference.trim().is_empty());
                let digest = if retained.is_some() {
                    binding.resolved_digest.clone()
                } else {
                    digest.clone()
                };
                let reference = match retained {
                    Some(reference) => reference.to_owned(),
                    None => reference.clone().ok_or_else(|| {
                        StackError::Validation(format!(
                            "Service '{}': Build has no successful deployable image.",
                            binding.service_name
                        ))
                    })?,
                };
                resolved.push(ResolvedStackBuildImageBinding {
                    service_name: binding.service_name.clone(),
                    build_project_id: binding.build_project_id,
                    image_reference: pin_to_digest(&reference, digest.as_deref()),
                    digest,
                    build_run_id: binding.resolved_build_run_id.or(*run_id),
                });
            }
            Ok(resolved)
        })
    }
}

fn first_reference(value: Value) -> Option<String> {
    value
        .as_array()?
        .iter()
        .find_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn pin_to_digest(reference: &str, digest: Option<&str>) -> String {
    let reference = reference.trim();
    let Some(digest) = digest.map(str::trim).filter(|digest| !digest.is_empty()) else {
        return reference.to_owned();
    };
    let base = if let Some((base, _)) = reference.rsplit_once('@') {
        base
    } else if let Some(index) = reference.rfind(':').filter(|index| {
        reference
            .rfind('/')
            .is_none_or(|last_slash| *index > last_slash)
    }) {
        &reference[..index]
    } else {
        reference
    };
    format!("{base}@{digest}")
}

fn storage(error: impl std::fmt::Display) -> StackError {
    StackError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a disposable CITADEL_WORKLOAD_DATABASE_URL"]
    async fn batched_bindings_preserve_service_order_pins_and_project_eligibility() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&std::env::var("CITADEL_WORKLOAD_DATABASE_URL").unwrap())
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TEMP TABLE buildprojects(id uuid, enabled bool, archivedat timestamptz);
            CREATE TEMP TABLE buildruns(id uuid,buildprojectid uuid,status text,registrysnapshot jsonb,completedat timestamptz,imagereferences jsonb,imagedigest text);
            INSERT INTO buildprojects VALUES(md5('project')::uuid,true,NULL);
            INSERT INTO buildruns SELECT md5(n::text)::uuid, id, 'Succeeded', '{}', now()+n*interval '1 day', jsonb_build_array('app:latest'), 'digest-'||n
            FROM buildprojects CROSS JOIN generate_series(1,2)n").execute(&pool).await.unwrap();
        let resolver = PostgresStackBuildImageResolver::new(pool.clone());
        let (project, older, latest): (Uuid, Uuid, Uuid) =
            sqlx::query_as("SELECT id, md5('1')::uuid, md5('2')::uuid FROM buildprojects")
                .fetch_one(&pool)
                .await
                .unwrap();
        let bindings:Vec<StackBuildImageBinding>=(0..25).map(|n|serde_json::from_value(serde_json::json!({
            "serviceName":format!("service-{n}"),"buildProjectId":project,"resolvedBuildRunId":(n%2==0).then_some(older)
        })).unwrap()).collect();
        let resolved = resolver.resolve(&bindings).await.unwrap();
        for (n, binding) in resolved.iter().enumerate() {
            assert_eq!(binding.service_name, bindings[n].service_name);
            assert_eq!(
                binding.build_run_id,
                Some(if n % 2 == 0 { older } else { latest })
            );
            assert_eq!(
                binding.image_reference,
                if n % 2 == 0 {
                    "app@digest-1"
                } else {
                    "app@digest-2"
                }
            );
        }
        assert_eq!(resolved.len(), bindings.len());
        sqlx::query("UPDATE buildprojects SET enabled=false")
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            resolver
                .resolve(&bindings)
                .await
                .unwrap_err()
                .to_string()
                .contains("service-0")
        );
    }
    #[test]
    fn pins_a_tag_without_corrupting_registry_ports() {
        assert_eq!(
            pin_to_digest("registry.test:5000/team/api:latest", Some("sha256:abc")),
            "registry.test:5000/team/api@sha256:abc"
        );
        assert_eq!(
            pin_to_digest("registry.test:5000/team/api", Some("sha256:abc")),
            "registry.test:5000/team/api@sha256:abc"
        );
    }
}
