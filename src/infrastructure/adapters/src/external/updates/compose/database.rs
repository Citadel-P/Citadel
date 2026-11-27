//! Qualified PostgreSQL target and the narrowly permitted installer changes.
use super::super::registration::fingerprint_inputs;
use super::{CheckedCoreReplacement, InstallerComposeInputs, publication::PreparedCoreReplacement};
use citadel_execution::ProcessRunner;
use citadel_updates::{DatabaseUpdatePlan, UpdateError};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroize;

#[derive(Clone, Copy)]
pub(in crate::external::updates) struct QualifiedDatabase {
    version: u32,
    image: &'static str,
}

pub(in crate::external::updates) const QUALIFIED: QualifiedDatabase = QualifiedDatabase {
    version: 190000,
    image: "postgres@sha256:cac45fb483caee3668946088a1c0532db89d5285c28dbfeac889b4068296e06f",
};

#[cfg(test)]
pub(in crate::external::updates) const SYNTHETIC: QualifiedDatabase = QualifiedDatabase {
    version: 190001,
    image: "postgres@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
};

impl QualifiedDatabase {
    pub(in crate::external::updates) fn update(
        self,
        source_version: u32,
    ) -> Result<Option<DatabaseUpdatePlan>, UpdateError> {
        citadel_primitives::postgres::require_supported(source_version)
            .map_err(UpdateError::Conflict)?;
        if source_version >= self.version {
            return Ok(None);
        }
        let plan = DatabaseUpdatePlan {
            source_version,
            target_version: self.version,
            target_image: self.image.into(),
        };
        plan.validate()?;
        Ok(Some(plan))
    }

    pub(in crate::external::updates) fn validate(
        self,
        plan: &DatabaseUpdatePlan,
    ) -> Result<(), UpdateError> {
        if self.update(plan.source_version)?.as_ref() != Some(plan) {
            return Err(unsupported());
        }
        Ok(())
    }
}

impl InstallerComposeInputs {
    pub(in crate::external::updates) async fn prepare_database_replacement(
        &self,
        qualified: QualifiedDatabase,
        plan: &DatabaseUpdatePlan,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<PreparedCoreReplacement, UpdateError> {
        qualified.validate(plan)?;
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
                "/var/lib/postgresql/19/docker",
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
                    != "/var/lib/postgresql/19/docker"
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
        for version in [190000, 190001, 190010] {
            assert_eq!(QUALIFIED.update(version).unwrap(), None);
        }
        for version in [0, 170011, 180007, 200000] {
            assert!(QUALIFIED.update(version).is_err());
        }
        let mut plan = SYNTHETIC.update(190000).unwrap().unwrap();
        assert!(QUALIFIED.validate(&plan).is_err());
        SYNTHETIC.validate(&plan).unwrap();
        plan.target_image = QUALIFIED.image.into();
        assert!(SYNTHETIC.validate(&plan).is_err());
    }
}
