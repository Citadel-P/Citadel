//! Version observations used by the updater; unknown Agents never imply Core-only.
use citadel_updates::UpdateError;
use sqlx::PgPool;
use std::time::Duration;

pub(crate) async fn read_update_agent_versions(pool: &PgPool) -> Result<Vec<String>, UpdateError> {
    let versions: Vec<Option<String>> = tokio::time::timeout(
        Duration::from_secs(5),
        sqlx::query_scalar(include_str!("agents.sql")).fetch_all(pool),
    )
    .await
    .map_err(|_| unavailable())?
    .map_err(|_| unavailable())?;
    checked_versions(versions)
}

fn checked_versions(versions: Vec<Option<String>>) -> Result<Vec<String>, UpdateError> {
    if versions.len() > 4096 {
        return Err(unavailable());
    }
    let mut checked = versions
        .into_iter()
        .map(|version| {
            let version = version.ok_or_else(unavailable)?;
            // Persisted handshake versions use the same canonical release identity as
            // Core. Development/unknown builds cannot be silently treated as releases.
            citadel_updates::release_version(&version).map_err(|_| unavailable())?;
            Ok(version)
        })
        .collect::<Result<Vec<_>, UpdateError>>()?;
    checked.sort();
    checked.dedup();
    Ok(checked)
}

fn unavailable() -> UpdateError {
    UpdateError::Unavailable("Cannot establish every registered Agent's published version. Reconcile disconnected or unobserved Agents before reviewing this update.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_versions_and_unbounded_inventory_do_not_mean_no_agents() {
        assert_eq!(checked_versions(vec![]).unwrap(), Vec::<String>::new());
        assert_eq!(
            checked_versions(vec![
                Some("0.1.2".into()),
                Some("0.1.1".into()),
                Some("0.1.2".into())
            ])
            .unwrap(),
            ["0.1.1", "0.1.2"]
        );
        for versions in [
            vec![None],
            vec![Some("".into())],
            vec![Some("0.1.1-dev.dirty".into())],
            vec![Some("0.1.1".into()); 4097],
        ] {
            assert!(checked_versions(versions).is_err());
        }
    }
}

#[cfg(test)]
mod database_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires a disposable CITADEL_UPDATE_TEST_DATABASE_URL named citadel_update_test_*"]
    async fn agent_inventory_includes_direct_edge_and_unobserved_swarm_installations() {
        let url = std::env::var("CITADEL_UPDATE_TEST_DATABASE_URL").unwrap();
        assert!(
            url::Url::parse(&url)
                .unwrap()
                .path()
                .starts_with("/citadel_update_test_")
        );
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        // Temporary tables shadow only these reads on this dedicated connection.
        // No application tables, identity, or stored credentials are modified.
        sqlx::raw_sql("CREATE TEMP TABLE platforms (connectortype text, agentversion text); CREATE TEMP TABLE edgeagentbindings (platformid integer,lastseenversion text,revokedatutc timestamptz,profile text DEFAULT 'Ordinary'); CREATE TEMP TABLE swarmnodeagentinstallations(platformid integer,desiredstate text DEFAULT 'Installed');")
            .execute(&pool).await.unwrap();
        sqlx::raw_sql("INSERT INTO platforms VALUES ('Local',NULL)")
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_update_agent_versions(&pool).await.unwrap().is_empty());
        sqlx::raw_sql("INSERT INTO platforms VALUES ('Agent',NULL)")
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_update_agent_versions(&pool).await.is_err());
        sqlx::raw_sql("UPDATE platforms SET agentversion='0.1.1' WHERE connectortype='Agent'; INSERT INTO edgeagentbindings(platformid,lastseenversion,revokedatutc) VALUES (1,NULL,NULL)").execute(&pool).await.unwrap();
        assert!(read_update_agent_versions(&pool).await.is_err());
        sqlx::raw_sql("UPDATE edgeagentbindings SET revokedatutc=now()")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(read_update_agent_versions(&pool).await.unwrap(), ["0.1.1"]);
        sqlx::raw_sql("INSERT INTO swarmnodeagentinstallations VALUES (2,'Removed')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(read_update_agent_versions(&pool).await.unwrap(), ["0.1.1"]);
        sqlx::raw_sql("INSERT INTO swarmnodeagentinstallations(platformid) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_update_agent_versions(&pool).await.is_err());
        sqlx::raw_sql("INSERT INTO edgeagentbindings VALUES (1,'0.1.2',NULL,'SwarmNode')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            read_update_agent_versions(&pool).await.unwrap(),
            ["0.1.1", "0.1.2"]
        );
        sqlx::raw_sql("INSERT INTO platforms SELECT 'Agent','0.1.1' FROM generate_series(1,4097)")
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_update_agent_versions(&pool).await.is_err());
        pool.close().await;
    }
}
