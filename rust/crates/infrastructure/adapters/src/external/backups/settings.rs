use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct BackupExecutionSettings {
    pub working_directory: PathBuf,
    pub allowed_core_paths: Vec<PathBuf>,
    pub default_timeout: Duration,
    pub maximum_log_line_bytes: usize,
}

impl BackupExecutionSettings {
    pub(crate) fn repository_path(&self, path: &str) -> Result<PathBuf, String> {
        let path = Path::new(path.trim());
        if path.as_os_str().is_empty() {
            return Err("Core repository path is required.".into());
        }
        let path = if path.is_absolute() {
            resolve(path)?
        } else {
            let root = self
                .allowed_core_paths
                .first()
                .ok_or("No allowed Core backup repository directory is configured.")?;
            resolve(&root.join(path))?
        };
        for allowed in &self.allowed_core_paths {
            if path.starts_with(resolve(allowed)?) {
                return Ok(path);
            }
        }
        Err("Core repository path is outside Backups__AllowedCorePaths.".into())
    }
}

// Resolve symlinks in existing ancestors, including when initializing a new repository.
fn resolve(path: &Path) -> Result<PathBuf, String> {
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Core repository paths cannot contain '..'.".into());
    }
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match std::fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for part in missing.iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if std::fs::symlink_metadata(ancestor)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err("Core repository path contains a dangling symlink.".into());
                }
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or("Invalid Core repository path")?
                        .to_owned(),
                );
                ancestor = ancestor.parent().ok_or("Invalid Core repository path")?;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn core_paths_use_directory_boundaries_and_resolve_symlinks() {
        let root =
            std::env::temp_dir().join(format!("citadel-backup-path-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(root.join("allowed")).unwrap();
        std::fs::create_dir_all(root.join("outside")).unwrap();
        let settings = BackupExecutionSettings {
            working_directory: root.clone(),
            allowed_core_paths: vec![root.join("allowed")],
            default_timeout: Duration::from_secs(120),
            maximum_log_line_bytes: 8192,
        };
        assert!(
            settings
                .repository_path(root.join("allowed/new/repository").to_str().unwrap())
                .is_ok()
        );
        assert_eq!(
            settings.repository_path(" daily/core ").unwrap(),
            root.join("allowed/daily/core")
        );
        for path in ["", "  ", "../outside", "daily/../../outside"] {
            assert!(settings.repository_path(path).is_err(), "{path}");
        }
        for path in ["allowed-other", "allowed/../outside"] {
            assert!(
                settings
                    .repository_path(root.join(path).to_str().unwrap())
                    .is_err()
            );
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("outside"), root.join("allowed/link")).unwrap();
            assert!(
                settings
                    .repository_path(root.join("allowed/link/new").to_str().unwrap())
                    .is_err()
            );
            assert!(settings.repository_path("link/new").is_err());
            std::os::unix::fs::symlink(root.join("missing"), root.join("allowed/dangling"))
                .unwrap();
            assert!(settings.repository_path("dangling/new").is_err());
        }
        let mut settings = settings;
        settings.allowed_core_paths.push(root.join("outside"));
        assert_eq!(
            settings.repository_path("daily/core").unwrap(),
            root.join("allowed/daily/core")
        );
        assert!(
            settings
                .repository_path(root.join("outside/repository").to_str().unwrap())
                .is_ok()
        );
        settings.allowed_core_paths = vec![root.join("not-created/repositories")];
        assert_eq!(
            settings.repository_path("daily/core").unwrap(),
            root.join("not-created/repositories/daily/core")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
