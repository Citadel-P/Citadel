//! Persist availability evidence and enqueue through the existing alert outbox.
use super::*;
use citadel_alerts::AlertObservation;
use citadel_updates::{AgentUpdateRelease, AgentUpdateTarget};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) async fn core(
    repository: &PostgresUpdateRepository,
    installed: &str,
    available: Option<&str>,
) -> Result<(), UpdateError> {
    let mut tx = repository.pool.begin().await.map_err(storage)?;
    let prior: Option<Option<Value>> =
        sqlx::query_scalar("SELECT availability FROM instanceupdatechecks WHERE id=1 FOR UPDATE")
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
    let previous = prior.flatten();
    let (matched, info) = if let Some(version) = available {
        if release_version(version)? <= release_version(installed)? {
            return Ok(());
        }
        (
            true,
            json!({"TargetVersion": version, "InstalledVersion": installed,
            "HumanMessage": format!("Citadel Core {version} is verified and eligible for this instance.")}),
        )
    } else {
        let Some(previous) = previous.as_ref() else {
            return Ok(());
        };
        let target = previous["info"]["TargetVersion"]
            .as_str()
            .unwrap_or_default();
        if release_version(target)? > release_version(installed)? || previous["matched"] != true {
            return Ok(());
        }
        {
            let mut info = previous["info"].clone();
            info["InstalledVersion"] = json!(installed);
            (false, info)
        }
    };
    let observation = observation(&mut tx, "CoreUpdateAvailable", matched, info).await?;
    sqlx::query("INSERT INTO instanceupdatechecks(id,availability) VALUES(1,$1) ON CONFLICT(id) DO UPDATE SET availability=$1")
        .bind(sqlx::types::Json(&observation)).execute(&mut *tx).await.map_err(storage)?;
    super::super::alerts::observations::enqueue(&mut tx, &observation)
        .await
        .map_err(storage)?;
    tx.commit().await.map_err(storage)
}

pub(super) async fn agents(
    repository: &PostgresUpdateRepository,
    release: &AgentUpdateRelease,
    targets: &[AgentUpdateTarget],
    observed_at: DateTime<Utc>,
) -> Result<(), UpdateError> {
    let mut tx = repository.pool.begin().await.map_err(storage)?;
    let row =
        sqlx::query("SELECT release,availability FROM agentupdatesettings WHERE id=1 FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
    let stored: Option<sqlx::types::Json<AgentUpdateRelease>> =
        row.try_get("release").map_err(storage)?;
    if stored.as_ref().map(|r| &r.0) != Some(release) {
        return Ok(());
    }
    let previous: Option<Value> = row.try_get("availability").map_err(storage)?;
    if previous
        .as_ref()
        .and_then(|value| value["observed_at"].as_str())
        .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
        .is_some_and(|at| at > observed_at)
    {
        return Ok(());
    }

    // Never expose daemon/container identity or connection details in alert payloads.
    let targets: Vec<_> = targets
        .iter()
        .cloned()
        .map(|mut target| {
            target.id = Sha256::digest(target.id.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            target
        })
        .collect();
    let (mut matched, mut info) = citadel_updates::agent_availability(
        release,
        &targets,
        previous
            .as_ref()
            .map(|v| &v["info"])
            .unwrap_or(&Value::Null),
    );
    // With no verified new candidates, keep tracking the previous advertised
    // release until its affected installations can be confirmed or are removed.
    if info["Count"] == 0
        && let Some(previous) = previous.as_ref().filter(|p| p["matched"] == true)
        && let Some(version) = previous["info"]["TargetVersion"]
            .as_str()
            .filter(|v| *v != release.version)
    {
        let previous_release = AgentUpdateRelease {
            version: version.into(),
            image: String::new(),
        };
        (matched, info) =
            citadel_updates::agent_availability(&previous_release, &targets, &previous["info"]);
    }
    let mut observation = observation(&mut tx, "AgentUpdatesAvailable", matched, info).await?;
    observation.observed_at = observed_at;
    sqlx::query("UPDATE agentupdatesettings SET availability=$1 WHERE id=1")
        .bind(sqlx::types::Json(&observation))
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    super::super::alerts::observations::enqueue(&mut tx, &observation)
        .await
        .map_err(storage)?;
    tx.commit().await.map_err(storage)
}

async fn observation(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    matched: bool,
    info: Value,
) -> Result<AlertObservation, UpdateError> {
    Ok(AlertObservation {
        alert_type: kind.into(),
        resource_id: instance(tx).await?,
        resource_type: "Updates".into(),
        resource_name: "Citadel updates".into(),
        deduplication_component: info["TargetVersion"].as_str().unwrap_or_default().into(),
        info,
        observed_at: Utc::now(),
        matched,
        value: None,
    })
}
