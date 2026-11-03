use citadel_backups::BackupError;
use std::{ffi::OsString, path::Path};
use url::Url;
type ProcessEnvironment = Vec<(OsString, OsString)>;
type PgDumpRequest = (Vec<OsString>, ProcessEnvironment);
pub(super) fn pg_dump_request(
    database_url: &str,
    destination: &Path,
) -> Result<PgDumpRequest, BackupError> {
    let environment = postgres_environment(database_url)?;
    Ok((
        vec![
            "--format=custom".into(),
            "--no-owner".into(),
            "--no-privileges".into(),
            "--file".into(),
            destination.as_os_str().to_owned(),
        ],
        environment,
    ))
}

pub(super) fn pg_restore_request(
    database_url: &str,
    source: &Path,
) -> Result<PgDumpRequest, BackupError> {
    Ok((
        vec![
            "--format=custom".into(),
            "--clean".into(),
            "--if-exists".into(),
            "--no-owner".into(),
            "--no-privileges".into(),
            "--single-transaction".into(),
            "--exit-on-error".into(),
            "--dbname".into(),
            // pg_restore interprets a --dbname value containing '=' as a libpq
            // connection string. Quote the database as a single conninfo value
            // so unusual names cannot override the host/user from our environment.
            format!(
                "dbname='{}'",
                postgres_database_name(database_url)?
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
            )
            .into(),
            source.as_os_str().to_owned(),
        ],
        postgres_environment(database_url)?,
    ))
}

pub(super) fn postgres_database_name(database_url: &str) -> Result<String, BackupError> {
    let url = postgres_url(database_url)?;
    decode_postgres_component(url.path().trim_start_matches('/'))
}

fn decode_postgres_component(value: &str) -> Result<String, BackupError> {
    let decoded = urlencoding::decode(value)
        .map_err(|_| BackupError::Validation("PostgreSQL URL contains invalid UTF-8.".into()))?;
    if decoded.contains('\0') {
        return Err(BackupError::Validation(
            "PostgreSQL URL contains a null byte.".into(),
        ));
    }
    Ok(decoded.into_owned())
}

fn postgres_environment(database_url: &str) -> Result<ProcessEnvironment, BackupError> {
    let url = postgres_url(database_url)?;
    let database = decode_postgres_component(url.path().trim_start_matches('/'))?;
    let mut environment = vec![
        ("PGHOST".into(), url.host_str().unwrap_or_default().into()),
        (
            "PGUSER".into(),
            decode_postgres_component(url.username())?.into(),
        ),
        ("PGDATABASE".into(), database.into()),
    ];
    if let Some(port) = url.port() {
        environment.push(("PGPORT".into(), port.to_string().into()));
    }
    if let Some(password) = url.password() {
        environment.push((
            "PGPASSWORD".into(),
            decode_postgres_component(password)?.into(),
        ));
    }
    if let Some((_, ssl_mode)) = url.query_pairs().find(|(key, _)| key == "sslmode") {
        environment.push(("PGSSLMODE".into(), ssl_mode.as_ref().into()));
    }
    Ok(environment)
}

fn postgres_url(database_url: &str) -> Result<Url, BackupError> {
    let url = Url::parse(database_url)
        .map_err(|_| BackupError::Validation("PostgreSQL URL is invalid.".into()))?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(BackupError::Validation(
            "PostgreSQL URL must use the postgres scheme.".into(),
        ));
    }
    if url.host_str().is_none()
        || url.username().is_empty()
        || url.path().trim_start_matches('/').is_empty()
    {
        return Err(BackupError::Validation(
            "PostgreSQL URL must include host, user, and database.".into(),
        ));
    }
    Ok(url)
}

pub(super) fn bounded_error(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).chars().take(4096).collect()
}
