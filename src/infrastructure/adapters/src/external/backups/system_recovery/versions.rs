//! Version evidence supplied by the configured PostgreSQL tools.
use citadel_backups::BackupError;
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest};
use citadel_primitives::postgres::{release_number, require_supported};
use std::{
    ffi::{OsStr, OsString},
    path::Path,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

async fn inspect(
    tool: &OsStr,
    args: Vec<OsString>,
    token: &CancellationToken,
) -> Result<String, BackupError> {
    let request = ProcessRequest::new(tool.to_owned())
        .args(args)
        .env("LC_ALL", "C")
        .limits(ProcessLimits {
            timeout: Duration::from_secs(30),
            maximum_stdout_bytes: 16 * 1024,
            maximum_stderr_bytes: 4096,
            output_limit_policy: OutputLimitPolicy::Truncate,
        });
    let output = citadel_processes::run(request, token)
        .await
        .map_err(|_| invalid())?;
    if !output.succeeded() || output.stdout_truncated {
        return Err(invalid());
    }
    String::from_utf8(output.stdout).map_err(|_| invalid())
}

fn invalid() -> BackupError {
    BackupError::Validation("Cannot verify PostgreSQL tool or archive version. Citadel requires PostgreSQL 19.x tools and a PG19 custom-format dump.".into())
}

pub(super) async fn tool_version(
    tool: &OsStr,
    name: &str,
    token: &CancellationToken,
) -> Result<u32, BackupError> {
    let info = inspect(tool, vec!["--version".into()], token).await?;
    let number = info
        .trim_end()
        .strip_prefix(&format!("{name} (PostgreSQL) "))
        .and_then(release_number)
        .ok_or_else(invalid)?;
    require_supported(number).map_err(BackupError::Validation)
}

pub(crate) async fn verify_backup_tools(
    dump: &OsStr,
    restore: &OsStr,
    token: &CancellationToken,
) -> Result<u32, BackupError> {
    let dumper = tool_version(dump, "pg_dump", token).await?;
    tool_version(restore, "pg_restore", token).await?;
    Ok(dumper)
}

pub(super) async fn verify_archive(
    tool: &OsStr,
    dump: &Path,
    server: u32,
    dumper: u32,
    token: &CancellationToken,
) -> Result<(), BackupError> {
    // Select no TOC entries. Even a large database only produces the archive header.
    let header = inspect(
        tool,
        vec![
            "--format=custom".into(),
            "--list".into(),
            "--use-list".into(),
            if cfg!(windows) { "NUL" } else { "/dev/null" }.into(),
            dump.into(),
        ],
        token,
    )
    .await?;
    for (label, expected) in [
        (";     Dumped from database version: ", server),
        (";     Dumped by pg_dump version: ", dumper),
    ] {
        let mut fields = header.lines().filter_map(|line| line.strip_prefix(label));
        if fields.next().and_then(release_number) != Some(expected) || fields.next().is_some() {
            return Err(BackupError::Validation(
                "PostgreSQL archive provenance differs from the recovery manifest or source."
                    .into(),
            ));
        }
    }
    Ok(())
}
