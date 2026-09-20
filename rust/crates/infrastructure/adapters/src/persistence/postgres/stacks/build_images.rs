use citadel_stacks::{
    ResolvedStackBuildImageBinding, StackBuildImageBinding, StackBuildImageResolverPort, StackError,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};

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
            let mut resolved = Vec::with_capacity(bindings.len());
            for binding in bindings {
                let row = sqlx::query(
                    r#"SELECT project.id AS projectid,run.id AS runid,run.imagereferences,run.imagedigest
FROM buildprojects project
LEFT JOIN LATERAL (
    SELECT candidate.id,candidate.imagereferences,candidate.imagedigest
    FROM buildruns candidate
    WHERE candidate.buildprojectid=project.id
      AND candidate.status='Succeeded'
      AND ($2::uuid IS NULL OR candidate.id=$2)
    ORDER BY candidate.completedat DESC NULLS LAST,candidate.id DESC
    LIMIT 1
) run ON TRUE
WHERE project.id=$1 AND project.enabled AND project.archivedat IS NULL"#,
                )
                .bind(binding.build_project_id)
                .bind(binding.resolved_build_run_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
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
                let run_id = row.try_get("runid").map_err(storage)?;
                let digest = if retained.is_some() {
                    binding.resolved_digest.clone()
                } else {
                    row.try_get("imagedigest").map_err(storage)?
                };
                let reference = match retained {
                    Some(reference) => reference.to_owned(),
                    None => first_reference(row.try_get("imagereferences").map_err(storage)?)
                        .ok_or_else(|| {
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
                    build_run_id: binding.resolved_build_run_id.or(run_id),
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
