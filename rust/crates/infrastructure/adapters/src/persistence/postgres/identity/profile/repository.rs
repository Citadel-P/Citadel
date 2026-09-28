use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_identity::{
    CurrentProfileRecord, IdentityError, PasswordChangeOutcome, ProfileRepository,
    ProfileResourceInfo, User, UserPreferences, UserPreferencesUpdate,
};
use citadel_identity::{
    UserAppearance, UserContentLayout, UserDateTimeFormat, UserTheme, UserThemeColor,
    UserUiDensity, UserUiFont, UserUiRadius,
};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
use crate::persistence::postgres::activities::store::invalid_activity;

const CURRENT_PROFILE_SQL: &str = r#"
WITH latest_oidc AS (
    SELECT DISTINCT ON (login.userid)
           login.userid,
           provider.id AS provider_id,
           provider.displayname AS provider_name
    FROM oidcexternallogins login
    JOIN oidcproviders provider ON provider.id = login.providerid
    ORDER BY login.userid, login.updatedat DESC
)
SELECT u.id,
       u.actorid,
       u.name,
       u.email,
       u.createdat,
       (u.password IS NOT NULL AND btrim(u.password) <> '') AS has_local_password,
       oidc.provider_id,
       oidc.provider_name,
       COALESCE(direct_roles.ids, ARRAY[]::uuid[]) AS role_ids,
       COALESCE(direct_roles.names, ARRAY[]::text[]) AS role_names,
       COALESCE(teams.ids, ARRAY[]::uuid[]) AS team_ids,
       COALESCE(teams.names, ARRAY[]::text[]) AS team_names
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User' AND actor.isenabled
LEFT JOIN latest_oidc oidc ON oidc.userid = u.id
LEFT JOIN LATERAL (
    SELECT array_agg(role.id ORDER BY role.name, role.id) AS ids,
           array_agg(role.name ORDER BY role.name, role.id) AS names
    FROM actorroles actor_role
    JOIN roles role ON role.id = actor_role.roleid
    WHERE actor_role.actorid = u.actorid
) direct_roles ON TRUE
LEFT JOIN LATERAL (
    SELECT array_agg(team.id ORDER BY team.name, team.id) AS ids,
           array_agg(team.name ORDER BY team.name, team.id) AS names
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    WHERE membership.memberactorid = u.actorid
) teams ON TRUE
WHERE u.id = $1
"#;

const USER_SQL: &str = r#"
SELECT u.id, u.name, u.email, u.password, u.actorid, u.createdbyactorid, u.createdat
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User' AND actor.isenabled
WHERE u.id = $1
"#;

const USER_PREFERENCES_SQL: &str = r#"
SELECT userid, timezone, datetimeformat, theme, updatedat, themecolor, font, radius, contentlayout, density
FROM userpreferences
WHERE userid = $1
"#;

#[derive(Clone)]
pub struct PostgresProfileRepository {
    pool: PgPool,
}

impl PostgresProfileRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ProfileRepository for PostgresProfileRepository {
    fn get_current(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<CurrentProfileRecord>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(CURRENT_PROFILE_SQL)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_profile)
                .transpose()
        })
    }

    fn get_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Option<User>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(USER_SQL)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_user)
                .transpose()
        })
    }

    fn rename_user<'a>(
        &'a self,
        user_id: Uuid,
        new_name: &'a str,
        actor_id: ActorId,
        renamed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<CurrentProfileRecord, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let old_name = lock_enabled_user(&mut transaction, user_id).await?;
            if old_name == new_name {
                let profile = sqlx::query(CURRENT_PROFILE_SQL)
                    .bind(user_id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?
                    .map(map_profile)
                    .transpose()?
                    .ok_or(IdentityError::NotFound)?;
                transaction.commit().await.map_err(storage)?;
                return Ok(profile);
            }
            let name_conflict = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM users WHERE id <> $1 AND lower(name) = lower($2))",
            )
            .bind(user_id)
            .bind(new_name)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if name_conflict {
                return Err(IdentityError::Conflict(
                    "A User with that name already exists.".to_owned(),
                ));
            }
            let updated = sqlx::query("UPDATE users SET name = $2 WHERE id = $1")
                .bind(user_id)
                .bind(new_name)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?
                .rows_affected();
            if updated != 1 {
                return Err(IdentityError::NotFound);
            }
            let activity = ActivityEvent::new_user_event(
                user_id,
                new_name.to_owned(),
                actor_id,
                ActivityEventInfo::user_profile_updated(old_name, new_name.to_owned()),
                renamed_at,
            )
            .map_err(invalid_activity)?;
            insert_activity(&mut transaction, &activity).await?;
            let profile = sqlx::query(CURRENT_PROFILE_SQL)
                .bind(user_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .map(map_profile)
                .transpose()?
                .ok_or(IdentityError::NotFound)?;
            transaction.commit().await.map_err(storage)?;
            Ok(profile)
        })
    }

    fn get_preferences(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserPreferences>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(USER_PREFERENCES_SQL)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_preferences)
                .transpose()
        })
    }

    fn patch_preferences<'a>(
        &'a self,
        user_id: Uuid,
        update: &'a UserPreferencesUpdate,
        updated_at: chrono::DateTime<chrono::Utc>,
        _actor_id: ActorId,
    ) -> BoxFuture<'a, Result<UserPreferences, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_enabled_user(&mut transaction, user_id).await?;
            let current = sqlx::query(USER_PREFERENCES_SQL)
                .bind(user_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .map(map_preferences)
                .transpose()?;
            let time_zone = update.time_zone.clone().unwrap_or_else(|| {
                current.as_ref().map_or_else(
                    || "UTC".to_owned(),
                    |preferences| preferences.time_zone().to_owned(),
                )
            });
            let date_time_format = update.date_time_format.unwrap_or_else(|| {
                current.as_ref().map_or(
                    UserDateTimeFormat::System,
                    UserPreferences::date_time_format,
                )
            });
            let theme = update.theme.unwrap_or_else(|| {
                current
                    .as_ref()
                    .map_or(UserTheme::System, UserPreferences::theme)
            });
            let theme_color = update.theme_color.unwrap_or_else(|| {
                current
                    .as_ref()
                    .map_or(UserThemeColor::default(), UserPreferences::theme_color)
            });
            let font = update.font.unwrap_or_else(|| {
                current
                    .as_ref()
                    .map_or(UserUiFont::Geist, UserPreferences::font)
            });
            let radius = update.radius.unwrap_or_else(|| {
                current
                    .as_ref()
                    .map_or(UserUiRadius::default(), UserPreferences::radius)
            });
            let content_layout = update.content_layout.unwrap_or_else(|| {
                current.as_ref().map_or(
                    UserContentLayout::default(),
                    UserPreferences::content_layout,
                )
            });
            let density = update.density.unwrap_or_else(|| {
                current
                    .as_ref()
                    .map_or(UserUiDensity::default(), UserPreferences::density)
            });
            // Personal preferences do not produce activity history. Keep no-op
            // detection so repeated saves also avoid rewriting the preference row.
            if current.as_ref().is_some_and(|preferences| {
                preferences.time_zone() == time_zone
                    && preferences.date_time_format() == date_time_format
                    && preferences.theme() == theme
                    && preferences.theme_color() == theme_color
                    && preferences.font() == font
                    && preferences.radius() == radius
                    && preferences.content_layout() == content_layout
                    && preferences.density() == density
            }) && let Some(current) = current
            {
                transaction.commit().await.map_err(storage)?;
                return Ok(current);
            }

            let mut preferences = current.unwrap_or_else(|| {
                UserPreferences::new(
                    user_id,
                    time_zone.clone(),
                    date_time_format,
                    theme,
                    updated_at,
                )
            });
            preferences.update(time_zone, date_time_format, theme, updated_at);
            preferences = preferences.with_appearance(UserAppearance {
                theme_color,
                font,
                radius,
                content_layout,
                density,
            });
            sqlx::query(
                r#"
INSERT INTO userpreferences (userid, timezone, datetimeformat, theme, updatedat, themecolor, font, radius, contentlayout, density)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
ON CONFLICT (userid) DO UPDATE
SET timezone = EXCLUDED.timezone,
    datetimeformat = EXCLUDED.datetimeformat,
    theme = EXCLUDED.theme,
    themecolor = EXCLUDED.themecolor,
    font = EXCLUDED.font,
    radius = EXCLUDED.radius,
    contentlayout = EXCLUDED.contentlayout,
    density = EXCLUDED.density,

    updatedat = EXCLUDED.updatedat
"#,
            )
            .bind(preferences.user_id())
            .bind(preferences.time_zone())
            .bind(preferences.date_time_format().as_database_str())
            .bind(preferences.theme().as_database_str())
            .bind(preferences.updated_at())
            .bind(preferences.theme_color().as_database_str())
            .bind(preferences.font().as_database_str())
            .bind(preferences.radius().as_database_str())
            .bind(preferences.content_layout().as_database_str())
            .bind(preferences.density().as_database_str())

            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(preferences)
        })
    }

    fn change_password<'a>(
        &'a self,
        user_id: Uuid,
        expected_password_hash: &'a str,
        new_password_hash: &'a str,
        current_session_id: Option<Uuid>,
        changed_at: chrono::DateTime<chrono::Utc>,
        actor_id: ActorId,
    ) -> BoxFuture<'a, Result<PasswordChangeOutcome, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query(
                r#"
SELECT u.password,
       u.name,
       EXISTS(SELECT 1 FROM oidcexternallogins login WHERE login.userid = u.id) AS externally_managed
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User' AND actor.isenabled
WHERE u.id = $1
FOR UPDATE OF u
"#,
            )
            .bind(user_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?
            .ok_or(IdentityError::NotFound)?;
            if row
                .try_get::<bool, _>("externally_managed")
                .map_err(storage)?
            {
                return Ok(PasswordChangeOutcome::ExternallyManaged);
            }
            if row
                .try_get::<Option<String>, _>("password")
                .map_err(storage)?
                .as_deref()
                != Some(expected_password_hash)
            {
                return Ok(PasswordChangeOutcome::CurrentPasswordMismatch);
            }
            let resource_name = row.try_get::<String, _>("name").map_err(storage)?;

            let updated = sqlx::query("UPDATE users SET password = $2 WHERE id = $1")
                .bind(user_id)
                .bind(new_password_hash)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?
                .rows_affected();
            if updated != 1 {
                return Err(IdentityError::NotFound);
            }
            sqlx::query(
                r#"
WITH retained_session AS MATERIALIZED (
    SELECT id
    FROM refreshtokens
    WHERE $2::uuid IS NOT NULL
      AND id = $2
      AND userid = $1
      AND expiresat > $3
    FOR UPDATE
)
DELETE FROM refreshtokens
WHERE userid = $1
  AND (
      NOT EXISTS (SELECT 1 FROM retained_session)
      OR id <> (SELECT id FROM retained_session)
  )
"#,
            )
            .bind(user_id)
            .bind(current_session_id)
            .bind(changed_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            let activity = ActivityEvent::new_user_event(
                user_id,
                resource_name,
                actor_id,
                ActivityEventInfo::user_password_changed(),
                changed_at,
            )
            .map_err(invalid_activity)?;
            insert_activity(&mut transaction, &activity).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(PasswordChangeOutcome::Changed)
        })
    }
}

async fn lock_enabled_user(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<String, IdentityError> {
    sqlx::query_scalar::<_, String>(
        r#"
SELECT u.name
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User' AND actor.isenabled
WHERE u.id = $1
FOR UPDATE OF u
"#,
    )
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    .ok_or(IdentityError::NotFound)
}

fn map_profile(row: sqlx::postgres::PgRow) -> Result<CurrentProfileRecord, IdentityError> {
    let role_ids = row.try_get::<Vec<Uuid>, _>("role_ids").map_err(storage)?;
    let role_names = row
        .try_get::<Vec<String>, _>("role_names")
        .map_err(storage)?;
    let team_ids = row.try_get::<Vec<Uuid>, _>("team_ids").map_err(storage)?;
    let team_names = row
        .try_get::<Vec<String>, _>("team_names")
        .map_err(storage)?;
    Ok(CurrentProfileRecord {
        id: row.try_get("id").map_err(storage)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        display_name: row.try_get("name").map_err(storage)?,
        email: required_email(&row)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        has_local_password: row.try_get("has_local_password").map_err(storage)?,
        oidc_provider_id: row.try_get("provider_id").map_err(storage)?,
        oidc_provider_name: row.try_get("provider_name").map_err(storage)?,
        direct_roles: map_resources(role_ids, role_names)?,
        teams: map_resources(team_ids, team_names)?,
    })
}

fn map_user(row: sqlx::postgres::PgRow) -> Result<User, IdentityError> {
    Ok(User::from_persistence(
        row.try_get("id").map_err(storage)?,
        row.try_get("name").map_err(storage)?,
        required_email(&row)?,
        row.try_get("password").map_err(storage)?,
        ActorId::new(row.try_get("actorid").map_err(storage)?),
        ActorId::new(row.try_get("createdbyactorid").map_err(storage)?),
        row.try_get("createdat").map_err(storage)?,
    ))
}

fn map_preferences(row: sqlx::postgres::PgRow) -> Result<UserPreferences, IdentityError> {
    let date_time_format = row
        .try_get::<String, _>("datetimeformat")
        .map_err(storage)?;
    let theme = row.try_get::<String, _>("theme").map_err(storage)?;
    Ok(UserPreferences::new(
        row.try_get("userid").map_err(storage)?,
        row.try_get("timezone").map_err(storage)?,
        UserDateTimeFormat::from_database_str(&date_time_format).ok_or_else(|| {
            IdentityError::Storage(format!(
                "Unknown User preference date/time format '{date_time_format}'."
            ))
        })?,
        UserTheme::from_database_str(&theme).ok_or_else(|| {
            IdentityError::Storage(format!("Unknown User preference theme '{theme}'."))
        })?,
        row.try_get("updatedat").map_err(storage)?,
    )
    .with_appearance(UserAppearance {
        theme_color: UserThemeColor::from_database_str(
            &row.try_get::<String, _>("themecolor").map_err(storage)?,
        )
        .ok_or_else(|| IdentityError::Storage("Unknown preference theme_color.".to_owned()))?,
        font: UserUiFont::from_database_str(&row.try_get::<String, _>("font").map_err(storage)?)
            .ok_or_else(|| IdentityError::Storage("Unknown preference font.".to_owned()))?,
        radius: UserUiRadius::from_database_str(
            &row.try_get::<String, _>("radius").map_err(storage)?,
        )
        .ok_or_else(|| IdentityError::Storage("Unknown preference radius.".to_owned()))?,
        content_layout: UserContentLayout::from_database_str(
            &row.try_get::<String, _>("contentlayout").map_err(storage)?,
        )
        .ok_or_else(|| IdentityError::Storage("Unknown preference content_layout.".to_owned()))?,
        density: UserUiDensity::from_database_str(
            &row.try_get::<String, _>("density").map_err(storage)?,
        )
        .ok_or_else(|| IdentityError::Storage("Unknown preference density.".to_owned()))?,
    }))
}

fn required_email(row: &sqlx::postgres::PgRow) -> Result<String, IdentityError> {
    row.try_get::<Option<String>, _>("email")
        .map_err(storage)?
        .ok_or_else(|| IdentityError::Storage("User email is missing.".to_owned()))
}

fn map_resources(
    ids: Vec<Uuid>,
    names: Vec<String>,
) -> Result<Vec<ProfileResourceInfo>, IdentityError> {
    if ids.len() != names.len() {
        return Err(IdentityError::Storage(
            "Profile resource projection is inconsistent.".to_owned(),
        ));
    }
    Ok(ids
        .into_iter()
        .zip(names)
        .map(|(id, name)| ProfileResourceInfo { id, name })
        .collect())
}

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
