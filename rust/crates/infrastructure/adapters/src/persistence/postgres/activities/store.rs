use citadel_activities::ActivityQueryStore;
use citadel_activities::ActivityRecord;
use citadel_activities::PagedActivityRecords;
use citadel_activities::ValidatedActivityFilter;
use citadel_activities::{ActivityAccess, ActivityError};
use citadel_activities::{
    ActivityEvent, ActivityEventType, ActivityInvariantError, ActivityResourceType, ActivityStatus,
};
use citadel_identity::ActorType;
use citadel_identity::IdentityError;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

const MAXIMUM_ACTIVITY_INFO_BYTES: usize = 256 * 1024;

mod volumes;
mod webhooks;

const AUTHORIZED_ACTIVITY_CTES: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), authorized_resources AS (
    SELECT DISTINCT permission.resourcetype, NULL::uuid AS resourceid
    FROM actorroles actor_role
    JOIN permissions permission ON permission.roleid = actor_role.roleid
    JOIN actor_scope scope ON scope.actorid = actor_role.actorid
    WHERE (permission.permissionlevel & 7) <> 0
    UNION
    SELECT DISTINCT access.resourcetype, access.resourceid
    FROM resourceaccesses access
    JOIN actor_scope scope ON scope.actorid = access.actorid
    WHERE (access.permissionlevel & 7) <> 0
), filtered_activities AS (
    SELECT activity.*
    FROM activityevents activity
    WHERE ($2::uuid IS NULL OR activity.resourceid = $2)
      AND ($3::text IS NULL OR activity.resourcetype = $3)
      AND ($4::text IS NULL OR activity.eventtype = $4)
      AND (
          $5::boolean
          OR EXISTS (
              SELECT 1
              FROM authorized_resources authorized
              WHERE authorized.resourcetype = CASE activity.resourcetype
                  WHEN 'Platform' THEN 0
                  WHEN 'Deployment' THEN 1
                  WHEN 'Stack' THEN 2
                  WHEN 'Registry' THEN 3
                  WHEN 'GitRepository' THEN 4
                  WHEN 'AlertRule' THEN 6
                  WHEN 'AutomationAction' THEN 13
                  WHEN 'BackupPolicy' THEN 16
                  WHEN 'Volume' THEN 0
                  WHEN 'Build' THEN 18
                  WHEN 'BuildAgentPool' THEN 19
                  WHEN 'SwarmService' THEN 20
                  WHEN 'ServiceAccount' THEN 21
              END
                AND (authorized.resourceid IS NULL OR authorized.resourceid = activity.resourceid)
          )
      )
)
"#;

const ACTIVITY_PROJECTION: &str = r#"
SELECT activity.id,
       activity.platformid,
       activity.resourceid,
       activity.resourcename,
       activity.resourcetype,
       activity.eventtype,
       activity.status,
       activity.info,
       activity.createdbyactorid,
       activity.createdat,
       COALESCE(platform.name, 'Unknown') AS platform_name,
       COALESCE(platform.status, 'Offline') AS platform_status,
       COALESCE(actor_user.name, service_account.name,
                CASE WHEN actor.type = 'System' THEN 'System' END,
                'Unknown') AS actor_name,
       actor.type AS actor_type
FROM filtered_activities activity
LEFT JOIN platforms platform ON platform.id = activity.platformid
JOIN actors actor ON actor.id = activity.createdbyactorid
LEFT JOIN users actor_user ON actor_user.actorid = actor.id
LEFT JOIN serviceaccounts service_account ON service_account.actorid = actor.id
"#;

#[derive(Clone)]
pub struct PostgresActivityStore {
    pool: PgPool,
}

impl PostgresActivityStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ActivityQueryStore for PostgresActivityStore {
    fn get_authorized<'a>(
        &'a self,
        principal: &'a ActivityAccess,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<ActivityRecord>, ActivityError>> {
        Box::pin(async move {
            let query = format!(
                "{AUTHORIZED_ACTIVITY_CTES}{ACTIVITY_PROJECTION} WHERE activity.id = $6 LIMIT 1"
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(principal.actor_id.value())
                .bind(Option::<Uuid>::None)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .bind(principal.administrator)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(activity_storage)?
                .map(|row| map_activity(row).map_err(activity_storage))
                .transpose()
        })
    }

    fn list_authorized<'a>(
        &'a self,
        principal: &'a ActivityAccess,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, ActivityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(activity_storage)?;
            let resource_type = filter
                .resource_type
                .map(|value| value.as_database_str().to_owned());
            let event_type = filter
                .event_type
                .map(|value| value.as_database_str().to_owned());
            let is_administrator = principal.administrator;
            let total_query =
                format!("{AUTHORIZED_ACTIVITY_CTES} SELECT COUNT(*) FROM filtered_activities");
            let total_count = sqlx::query_scalar::<_, i64>(AssertSqlSafe(total_query.as_str()))
                .bind(principal.actor_id.value())
                .bind(filter.resource_id)
                .bind(resource_type.as_deref())
                .bind(event_type.as_deref())
                .bind(is_administrator)
                .fetch_one(&mut *transaction)
                .await
                .map_err(activity_storage)?;

            let page_query = format!(
                "{AUTHORIZED_ACTIVITY_CTES}{ACTIVITY_PROJECTION} ORDER BY activity.createdat DESC, activity.id DESC LIMIT $6 OFFSET $7"
            );
            let offset = i64::from(filter.page - 1) * i64::from(filter.page_size);
            let rows = sqlx::query(AssertSqlSafe(page_query.as_str()))
                .bind(principal.actor_id.value())
                .bind(filter.resource_id)
                .bind(resource_type.as_deref())
                .bind(event_type.as_deref())
                .bind(is_administrator)
                .bind(filter.page_size)
                .bind(offset)
                .fetch_all(&mut *transaction)
                .await
                .map_err(activity_storage)?;
            transaction.commit().await.map_err(activity_storage)?;
            let items = rows
                .into_iter()
                .map(|row| map_activity(row).map_err(activity_storage))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PagedActivityRecords {
                items,
                total_count,
                page: filter.page,
                page_size: filter.page_size,
            })
        })
    }
}

pub(crate) async fn insert_activity(
    transaction: &mut Transaction<'_, Postgres>,
    activity: &ActivityEvent,
) -> Result<(), IdentityError> {
    let mut info = serde_json::to_value(activity.info()).map_err(storage)?;
    redact_webhook_credentials(&mut info);
    let info_json = serde_json::to_string(&info).map_err(storage)?;
    if info_json.len() > MAXIMUM_ACTIVITY_INFO_BYTES {
        return Err(IdentityError::Storage(
            "Activity info exceeded the persisted size limit.".to_owned(),
        ));
    }
    let affected = sqlx::query(
        r#"
INSERT INTO activityevents (
    id, platformid, resourceid, resourcename, resourcetype,
    eventtype, info, createdbyactorid, status, createdat
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
"#,
    )
    .bind(activity.id())
    .bind(activity.platform_id())
    .bind(activity.resource_id())
    .bind(activity.resource_name())
    .bind(activity.resource_type().as_database_str())
    .bind(activity.event_type().as_database_str())
    .bind(info_json)
    .bind(activity.created_by_actor_id().value())
    .bind(activity.status().as_database_str())
    .bind(activity.created_at())
    .execute(&mut **transaction)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        return Err(IdentityError::Storage(
            "Activity insert did not affect exactly one row.".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn invalid_activity(error: ActivityInvariantError) -> IdentityError {
    IdentityError::Storage(format!("Invalid Activity event: {error}"))
}

// Configuration snapshots are audit data, not a second credential store. Apply
// this once at the persistence boundary for old/new Stack, Service, Build and
// Automation snapshots; leave the actual resource configuration untouched.
fn redact_webhook_credentials(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name.eq_ignore_ascii_case("webhook")
                    && let Value::Object(webhook) = value
                {
                    for (key, value) in webhook {
                        if key.eq_ignore_ascii_case("secret")
                            && value.as_str().is_some_and(|s| !s.is_empty())
                        {
                            *value = Value::String("********".into());
                        }
                    }
                } else {
                    redact_webhook_credentials(value);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_webhook_credentials),
        _ => {}
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn audit_snapshots_mask_webhook_credentials_without_losing_configuration_metadata() {
        let mut snapshot = json!({"Old":{"StackRelease":{"Spec":{"Webhook":{"Enabled":true,"Provider":"Generic","Secret":"old-secret"}}}},
            "New":{"spec":{"webhook":{"enabled":true,"secret":"new-secret"}}},"references":[{"Webhook":{"Secret":null}}]});
        redact_webhook_credentials(&mut snapshot);
        assert!(!snapshot.to_string().contains("old-secret"));
        assert!(!snapshot.to_string().contains("new-secret"));
        assert_eq!(
            snapshot["Old"]["StackRelease"]["Spec"]["Webhook"]["Provider"],
            "Generic"
        );
        assert_eq!(snapshot["New"]["spec"]["webhook"]["enabled"], true);
        assert!(snapshot["references"][0]["Webhook"]["Secret"].is_null());
    }
}

fn map_activity(row: sqlx::postgres::PgRow) -> Result<ActivityRecord, IdentityError> {
    let resource_type = row.try_get::<String, _>("resourcetype").map_err(storage)?;
    let event_type = row.try_get::<String, _>("eventtype").map_err(storage)?;
    let status = row.try_get::<String, _>("status").map_err(storage)?;
    let actor_type = row.try_get::<String, _>("actor_type").map_err(storage)?;
    let event_type = ActivityEventType::from_database_str(&event_type).ok_or_else(|| {
        IdentityError::Storage(format!(
            "Unknown persisted ActivityEventType '{event_type}'."
        ))
    })?;
    let resource_type =
        ActivityResourceType::from_database_str(&resource_type).ok_or_else(|| {
            IdentityError::Storage(format!(
                "Unknown persisted ActivityResourceType '{resource_type}'."
            ))
        })?;
    if event_type.resource_type() != resource_type {
        return Err(IdentityError::Storage(format!(
            "Activity event '{}' does not match resource type '{}'.",
            event_type.as_database_str(),
            resource_type.as_database_str()
        )));
    }
    let info_json = row.try_get::<String, _>("info").map_err(storage)?;
    validate_persisted_info(&info_json, event_type)?;
    Ok(ActivityRecord {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        resource_id: row.try_get("resourceid").map_err(storage)?,
        platform_name: row.try_get("platform_name").map_err(storage)?,
        resource_name: row.try_get("resourcename").map_err(storage)?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        resource_type,
        event_type,
        status: ActivityStatus::from_database_str(&status).ok_or_else(|| {
            IdentityError::Storage(format!("Unknown persisted ActivityStatus '{status}'."))
        })?,
        created_at: row.try_get("createdat").map_err(storage)?,
        info_json,
        actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        actor_name: row.try_get("actor_name").map_err(storage)?,
        actor_type: ActorType::from_database_str(&actor_type)
            .ok_or_else(|| {
                IdentityError::Storage(format!("Unknown persisted ActorType '{actor_type}'."))
            })?
            .as_database_str()
            .to_owned(),
    })
}

fn validate_persisted_info(
    info_json: &str,
    event_type: ActivityEventType,
) -> Result<(), IdentityError> {
    if info_json.len() > MAXIMUM_ACTIVITY_INFO_BYTES {
        return Err(IdentityError::Storage(
            "Persisted Activity info exceeded the read size limit.".to_owned(),
        ));
    }
    let value = serde_json::from_str::<Value>(info_json).map_err(storage)?;
    let discriminator = value
        .as_object()
        .and_then(|object| object.get("$type"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            IdentityError::Storage("Persisted Activity info has no discriminator.".to_owned())
        })?;
    if discriminator != event_type.as_database_str() {
        return Err(IdentityError::Storage(format!(
            "Activity info discriminator '{discriminator}' does not match event type '{}'.",
            event_type.as_database_str()
        )));
    }
    Ok(())
}

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use citadel_activities::ActivityEventInfo;
    use citadel_primitives::ActorId;
    use serde_json::json;

    #[test]
    fn rust_profile_info_matches_the_dotnet_persistence_contract() {
        let info =
            ActivityEventInfo::user_profile_updated("Old owner".to_owned(), "New owner".to_owned());

        assert_eq!(
            serde_json::to_value(info).unwrap(),
            json!({
                "$type": "UserProfileUpdated",
                "Changes": [{
                    "Name": "DisplayName",
                    "OldValue": "Old owner",
                    "NewValue": "New owner"
                }]
            })
        );
    }

    #[test]
    fn password_activity_contains_no_sensitive_payload() {
        let event = ActivityEvent::new_user_event(
            Uuid::now_v7(),
            "Owner".to_owned(),
            ActorId::new(Uuid::now_v7()),
            ActivityEventInfo::user_password_changed(),
            Utc::now(),
        )
        .unwrap();
        let json = serde_json::to_string(event.info()).unwrap();

        assert_eq!(json, r#"{"$type":"UserPasswordChanged"}"#);
    }

    #[test]
    fn legacy_info_discriminator_must_match_the_row() {
        let error = validate_persisted_info(
            r#"{"$type":"UserPasswordChanged"}"#,
            ActivityEventType::UserSessionRevoked,
        )
        .unwrap_err();

        assert!(error.to_string().contains("does not match"));
    }
}

fn activity_storage(error: impl std::fmt::Display) -> ActivityError {
    ActivityError::Storage(error.to_string())
}
