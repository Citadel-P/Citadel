//! Qualified PostgreSQL target and the narrowly permitted installer changes.
use super::super::registration::fingerprint_inputs;
use super::{CheckedCoreReplacement, InstallerComposeInputs, publication::PreparedCoreReplacement};
use citadel_execution::ProcessRunner;
use citadel_updates::{DatabaseUpdatePlan, UpdateError};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroize;

const TARGET_VERSION: u32 = 180006;
const TARGET_IMAGE: &str =
    "postgres@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5";

pub(in crate::external::updates) fn database_update(
    source_version: u32,
) -> Result<Option<DatabaseUpdatePlan>, UpdateError> {
    if source_version / 10000 != 18 {
        return Err(unsupported());
    }
    if source_version >= TARGET_VERSION {
        return Ok(None);
    }
    let plan = DatabaseUpdatePlan {
        source_version,
        target_version: TARGET_VERSION,
        target_image: TARGET_IMAGE.into(),
    };
    plan.validate()?;
    Ok(Some(plan))
}

pub(in crate::external::updates) fn validate_database_update(
    plan: &DatabaseUpdatePlan,
) -> Result<(), UpdateError> {
    if database_update(plan.source_version)?.as_ref() != Some(plan) {
        return Err(unsupported());
    }
    Ok(())
}

impl InstallerComposeInputs {
    pub(in crate::external::updates) async fn prepare_database_replacement(
        &self,
        plan: &DatabaseUpdatePlan,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<PreparedCoreReplacement, UpdateError> {
        validate_database_update(plan)?;
        super::publication::check_environment(self)?;
        let mut captured = self.capture()?;
        let result = async {
            let source_hash = fingerprint_inputs(&self.project, &captured);
            let source = self
                .resolve_snapshot(&captured, process, cancellation)
                .await?;
            let checked_source = self.validate_resolved(&source, source_hash.clone())?;
            let source_data = format!("/var/lib/postgresql/{}/docker", plan.source_version / 10000);
            if source["services"]["pg_db"]["environment"]["PGDATA"] != source_data {
                return Err(unsupported());
            }
            let original = &captured.last().ok_or_else(unsupported)?.1;
            let environment =
                super::replacement::replace_selector(original, "PG_IMAGE", &plan.target_image)?;
            let environment = super::replacement::replace_selector(
                &environment,
                "PG_DATA",
                "/var/lib/postgresql/18/docker",
            )?;
            let bytes = &mut captured.last_mut().ok_or_else(unsupported)?.1;
            bytes.zeroize();
            *bytes = environment.to_vec();
            let target_hash = fingerprint_inputs(&self.project, &captured);
            let mut target = self
                .resolve_snapshot(&captured, process, cancellation)
                .await?;
            self.validate_resolved(&target, target_hash.clone())?;
            if target_hash == source_hash
                || target["services"]["pg_db"]["image"] != plan.target_image
                || target["services"]["pg_db"]["environment"]["PGDATA"]
                    != "/var/lib/postgresql/18/docker"
            {
                return Err(unsupported());
            }
            target["services"]["pg_db"]["image"] = source["services"]["pg_db"]["image"].clone();
            target["services"]["pg_db"]["environment"]["PGDATA"] =
                source["services"]["pg_db"]["environment"]["PGDATA"].clone();
            // The installer passes the raw environment to Core. Only the two
            // engine selectors may change there; credentials stay identical.
            for key in ["PG_IMAGE", "PG_DATA"] {
                let environment = target["services"]["server"]["environment"]
                    .as_object_mut()
                    .ok_or_else(unsupported)?;
                match source["services"]["server"]["environment"].get(key) {
                    Some(value) => {
                        environment.insert(key.into(), value.clone());
                    }
                    None => {
                        environment.remove(key);
                    }
                }
            }
            if source != target
                || fingerprint_inputs(&self.project, &self.capture()?) != source_hash
            {
                return Err(unsupported());
            }
            Ok(PreparedCoreReplacement {
                checked: CheckedCoreReplacement {
                    source_configuration_sha256: source_hash,
                    target_configuration_sha256: target_hash,
                    target_image: checked_source.core_image,
                },
                environment,
            })
        }
        .await;
        for (_, bytes) in &mut captured {
            bytes.zeroize();
        }
        result
    }
}
fn unsupported() -> UpdateError {
    UpdateError::Conflict("PostgreSQL replacement must use the qualified image and PG_IMAGE/PG_DATA selectors without changing other installation settings.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_qualified_engine_target_can_be_selected() {
        for source in [180000, 180001, 180005] {
            let plan = database_update(source).unwrap().unwrap();
            validate_database_update(&plan).unwrap();
            let mut changed = plan.clone();
            changed.target_image = format!("postgres@sha256:{}", "a".repeat(64));
            assert!(validate_database_update(&changed).is_err());
            changed = plan;
            changed.target_version = 180007;
            assert!(validate_database_update(&changed).is_err());
        }
        assert!(database_update(160010).is_err());
        assert!(database_update(170011).is_err());
        assert!(database_update(180006).unwrap().is_none());
        assert!(database_update(190000).is_err());
    }
}
