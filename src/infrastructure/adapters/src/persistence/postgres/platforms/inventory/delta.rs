//! Targeted resource writes, sharing the snapshot generation fence and row locks.
use super::store::{self, PostgresInventoryProjectionStore, storage};
use citadel_platforms::{
    RuntimeCapabilityError,
    jobs::{ProjectionChange, ProjectionWrite, ResourceDelta, SnapshotGeneration},
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

impl PostgresInventoryProjectionStore {
    pub async fn persist_delta(
        &self,
        platform: Uuid,
        delta: &ResourceDelta,
        generation: &SnapshotGeneration,
    ) -> Result<ProjectionChange, RuntimeCapabilityError> {
        let write = ProjectionWrite::begin(platform, None, delta.projection_kind()).await;
        if !generation.matches(&write) {
            return Ok(ProjectionChange::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let changed = persist_delta(&mut tx, platform, None, delta).await?;
        tx.commit().await.map_err(storage)?;
        write.committed();
        Ok(ProjectionChange::committed(changed))
    }
}

pub(crate) async fn persist_delta(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    node: Option<&str>,
    delta: &ResourceDelta,
) -> Result<bool, RuntimeCapabilityError> {
    use crate::persistence::postgres::platforms::nodes::store as nodes;
    store::validate_runtime_ids([delta.id()])?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    let now = chrono::Utc::now();
    if let Some(node) = node {
        return match delta {
            ResourceDelta::Image {
                value: Some(value), ..
            } => {
                nodes::persist_images_observations(
                    tx,
                    platform,
                    node,
                    now,
                    std::slice::from_ref(value),
                    false,
                )
                .await
            }
            ResourceDelta::Network {
                value: Some(value), ..
            } => {
                nodes::persist_networks_observations(
                    tx,
                    platform,
                    node,
                    now,
                    std::slice::from_ref(value),
                    false,
                )
                .await
            }
            ResourceDelta::Volume {
                value: Some(value), ..
            } => {
                nodes::persist_volumes_observations(
                    tx,
                    platform,
                    node,
                    now,
                    std::slice::from_ref(value),
                    false,
                )
                .await
            }
            _ => {
                let query = match delta {
                    ResourceDelta::Image { .. } => {
                        "DELETE FROM swarmnodeimageprojections WHERE platformid=$1 AND dockernodeid=$2 AND dockerimageid=$3 AND observedat<=$4"
                    }
                    ResourceDelta::Network { .. } => {
                        "DELETE FROM swarmnodenetworkprojections WHERE platformid=$1 AND dockernodeid=$2 AND dockernetworkid=$3 AND observedat<=$4"
                    }
                    ResourceDelta::Volume { .. } => {
                        "DELETE FROM swarmnodevolumeprojections WHERE platformid=$1 AND dockernodeid=$2 AND volumename=$3 AND observedat<=$4"
                    }
                };
                Ok(sqlx::query(query)
                    .bind(platform)
                    .bind(node)
                    .bind(delta.id())
                    .bind(now)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)?
                    .rows_affected()
                    > 0)
            }
        };
    }
    match delta {
        ResourceDelta::Image { value, .. } => {
            let mut changed = if let Some(value) = value {
                store::persist_image_observations(tx, platform, std::slice::from_ref(value), false)
                    .await?
            } else {
                sqlx::query("DELETE FROM images WHERE platformid=$1 AND dockerimageid=$2")
                    .bind(platform)
                    .bind(delta.id())
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)?
                    .rows_affected()
                    > 0
            };
            changed |= sqlx::query("UPDATE containers c SET imageid=i.id FROM images i WHERE c.platformid=$1 AND i.platformid=c.platformid AND i.dockerimageid=c.dockerimageid AND c.imageid IS DISTINCT FROM i.id")
                .bind(platform).execute(&mut **tx).await.map_err(storage)?.rows_affected()>0;
            sqlx::query("UPDATE platforms SET imagecount=(SELECT count(*) FROM images WHERE platformid=$1) WHERE id=$1 AND imagecount IS DISTINCT FROM (SELECT count(*) FROM images WHERE platformid=$1)")
                .bind(platform).execute(&mut **tx).await.map_err(storage)?;
            Ok(changed)
        }
        // Standalone network/volume inventories are live, not durable sets. The
        // accepted payload is published after commit; regular stats repair counts.
        ResourceDelta::Network { .. } | ResourceDelta::Volume { .. } => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires CITADEL_TEST_DATABASE_URL"]
    async fn resource_deltas_keep_siblings_replay_idempotently_and_fence_older_snapshots() {
        use citadel_platforms::{
            RuntimeImageSummary, RuntimeNetworkSummary, RuntimeVolumeSummary,
            jobs::{ProjectionKind, ResourceInventory, ResourceSnapshot},
        };
        let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
        citadel_database::MigrationRunner::migrate(&url)
            .await
            .unwrap();
        let pool = sqlx::PgPool::connect(&url).await.unwrap();
        let platform = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{}','Online',0)").bind(platform).bind(platform.to_string()).execute(&pool).await.unwrap();
        let store = PostgresInventoryProjectionStore::new(pool.clone());
        for id in ["first", "sibling"] {
            let value = RuntimeImageSummary {
                id: id.into(),
                ..Default::default()
            };
            let stamp = SnapshotGeneration::capture(platform, None, ProjectionKind::Images).await;
            assert!(
                store
                    .persist_delta(
                        platform,
                        &ResourceDelta::Image {
                            id: id.into(),
                            value: Some(value.clone())
                        },
                        &stamp
                    )
                    .await
                    .unwrap()
                    .changed()
            );
            let stamp = SnapshotGeneration::capture(platform, None, ProjectionKind::Images).await;
            assert!(
                !store
                    .persist_delta(
                        platform,
                        &ResourceDelta::Image {
                            id: id.into(),
                            value: Some(value)
                        },
                        &stamp
                    )
                    .await
                    .unwrap()
                    .changed()
            );
        }
        let old = SnapshotGeneration::capture(platform, None, ProjectionKind::Images).await;
        let stamp = SnapshotGeneration::capture(platform, None, ProjectionKind::Images).await;
        let delta = ResourceDelta::Image {
            id: "first".into(),
            value: None,
        };
        assert!(
            store
                .persist_delta(platform, &delta, &stamp)
                .await
                .unwrap()
                .changed()
        );
        assert!(
            !store
                .persist_resource_checked(
                    &ResourceSnapshot {
                        platform_id: platform,
                        observed_at: chrono::Utc::now(),
                        inventory: ResourceInventory::Images(vec![])
                    },
                    Some(&old)
                )
                .await
                .unwrap()
        );
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT dockerimageid FROM images WHERE platformid=$1")
                .bind(platform)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(ids, ["sibling"]);
        for node in ["a", "b"] {
            for id in ["one", "two"] {
                let mut tx = pool.begin().await.unwrap();
                for delta in [
                    ResourceDelta::Image {
                        id: id.into(),
                        value: Some(RuntimeImageSummary {
                            id: id.into(),
                            ..Default::default()
                        }),
                    },
                    ResourceDelta::Network {
                        id: id.into(),
                        value: Some(RuntimeNetworkSummary {
                            id: id.into(),
                            ..Default::default()
                        }),
                    },
                    ResourceDelta::Volume {
                        id: id.into(),
                        value: Some(RuntimeVolumeSummary {
                            name: id.into(),
                            ..Default::default()
                        }),
                    },
                ] {
                    assert!(
                        persist_delta(&mut tx, platform, Some(node), &delta)
                            .await
                            .unwrap()
                    );
                    assert!(
                        !persist_delta(&mut tx, platform, Some(node), &delta)
                            .await
                            .unwrap()
                    );
                }
                tx.commit().await.unwrap();
            }
        }
        let mut tx = pool.begin().await.unwrap();
        for delta in [
            ResourceDelta::Image {
                id: "one".into(),
                value: None,
            },
            ResourceDelta::Network {
                id: "one".into(),
                value: None,
            },
            ResourceDelta::Volume {
                id: "one".into(),
                value: None,
            },
        ] {
            assert!(
                persist_delta(&mut tx, platform, Some("a"), &delta)
                    .await
                    .unwrap()
            );
            assert!(
                !persist_delta(&mut tx, platform, Some("a"), &delta)
                    .await
                    .unwrap()
            );
        }
        tx.commit().await.unwrap();
        for sql in [
            "SELECT count(*) FROM swarmnodeimageprojections WHERE platformid=$1",
            "SELECT count(*) FROM swarmnodenetworkprojections WHERE platformid=$1",
            "SELECT count(*) FROM swarmnodevolumeprojections WHERE platformid=$1",
        ] {
            assert_eq!(
                sqlx::query_scalar::<_, i64>(sql)
                    .bind(platform)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                3
            );
        }
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
}
