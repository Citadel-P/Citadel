use super::*;

impl ReleaseCheckRepository for PostgresUpdateRepository {
    fn database_version(&self) -> BoxFuture<'_, Result<u32, UpdateError>> {
        Box::pin(async {
            let read =
                sqlx::query_scalar::<_, String>("SHOW server_version_num").fetch_one(&self.pool);
            let version = tokio::time::timeout(std::time::Duration::from_secs(5), read)
                .await
                .map_err(|_| UpdateError::Unavailable("Database version is unavailable.".into()))?
                .map_err(storage)?;
            version
                .parse()
                .map_err(|_| UpdateError::Unavailable("Database version is unavailable.".into()))
        })
    }

    fn observe_core_availability<'a>(
        &'a self,
        installed: &'a str,
        available: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(super::availability::core(self, installed, available))
    }

    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>> {
        Box::pin(async {
            let row = sqlx::query("SELECT lastattempt,lastsuccess,error,available FROM instanceupdatechecks WHERE id=1").fetch_optional(&self.pool).await.map_err(storage)?;
            row.map(|r| {
                Ok(CheckState {
                    last_attempt: r.try_get("lastattempt").map_err(storage)?,
                    last_success: r.try_get("lastsuccess").map_err(storage)?,
                    error: r.try_get("error").map_err(storage)?,
                    available: r
                        .try_get::<Option<sqlx::types::Json<ReleaseNotice>>, _>("available")
                        .map_err(storage)?
                        .map(|v| v.0),
                })
            })
            .unwrap_or(Ok(CheckState::default()))
        })
    }

    fn record_check<'a>(
        &'a self,
        result: Result<Option<&'a ReleaseNotice>, &'a str>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let instance = super::super::licensing::store::get_or_create_identity(&mut tx, now)
                .await
                .map_err(storage)?
                .instance_id;
            let mut discovered = false;
            match result {
                Ok(notice) => {
                    if let Some(notice) = notice {
                        let version = release_version(&notice.version)?;
                        if notice.release_url
                            != format!(
                                "https://github.com/Citadel-P/Citadel/releases/tag/v{version}"
                            )
                        {
                            return Err(UpdateError::Validation(
                                "Invalid official release identity.".into(),
                            ));
                        }
                        discovered = sqlx::query("INSERT INTO instanceupdatediscoveries(instanceid,version,discoveredat) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
                            .bind(instance).bind(&notice.version).bind(now).execute(&mut *tx).await.map_err(storage)?.rows_affected() == 1;
                        if discovered {
                            activity(
                                &mut tx,
                                instance,
                                ActorId::new(SYSTEM_ACTOR_ID),
                                ActivityEventInfo::UpdateDiscovered {
                                    version: notice.version.clone(),
                                    release_url: notice.release_url.clone(),
                                },
                                ActivityStatus::Information,
                                now,
                            )
                            .await?;
                        }
                    }
                    sqlx::query("INSERT INTO instanceupdatechecks(id,lastattempt,lastsuccess,available) VALUES(1,$1,$1,$2) ON CONFLICT(id) DO UPDATE SET lastattempt=$1,lastsuccess=$1,error=NULL,available=$2")
                        .bind(now).bind(notice.map(sqlx::types::Json)).execute(&mut *tx).await.map_err(storage)?;
                }
                Err(_) => {
                    // Never persist transport errors, URLs, headers or credentials.
                    sqlx::query("INSERT INTO instanceupdatechecks(id,lastattempt,error) VALUES(1,$1,$2) ON CONFLICT(id) DO UPDATE SET lastattempt=$1,error=$2")
                        .bind(now).bind("The official release check failed. Try again later.").execute(&mut *tx).await.map_err(storage)?;
                }
            }
            tx.commit().await.map_err(storage)?;
            Ok(discovered)
        })
    }
}
