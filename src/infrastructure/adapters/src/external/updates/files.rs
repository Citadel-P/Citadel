use citadel_updates::UpdateError;
use std::path::{Component, Path, PathBuf};

pub(super) fn validate_absolute(path: &Path) -> Result<(), UpdateError> {
    if !path.is_absolute()
        || path.components().collect::<PathBuf>().as_os_str() != path.as_os_str()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(UpdateError::Validation(
            "Update paths must be canonical absolute paths.".into(),
        ));
    }
    Ok(())
}

pub(super) fn mapped_path(root: &Path, path: &Path) -> Result<PathBuf, UpdateError> {
    validate_absolute(path)?;
    Ok(root.join(
        path.strip_prefix("/")
            .map_err(|_| UpdateError::Validation("Invalid host path.".into()))?,
    ))
}

pub(super) fn read_bounded(mut file: std::fs::File, maximum: u64) -> Result<Vec<u8>, UpdateError> {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata().map_err(|_| unavailable_input())?;
    if !metadata.is_file() || metadata.len() > maximum || metadata.mode() & 0o022 != 0 {
        return Err(unavailable_input());
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable_input())?;
    if bytes.len() as u64 > maximum {
        return Err(unavailable_input());
    }
    Ok(bytes)
}

pub(super) fn unavailable_input() -> UpdateError {
    UpdateError::Unavailable("Update input is missing, unsafe or exceeds its size limit.".into())
}

/// Walk anchored directory descriptors: replacing an ancestor with a symlink
/// cannot redirect a privileged read between path validation and open.
pub(super) fn open_secure(path: &Path, directory: bool) -> Result<std::fs::File, UpdateError> {
    use rustix::fs::{Mode, OFlags, openat};
    use std::os::unix::fs::MetadataExt;
    validate_absolute(path)?;
    let mut parent = std::fs::File::open("/").map_err(|_| unavailable_input())?;
    let parts = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>();
    for (index, name) in parts.iter().enumerate() {
        let is_directory = index + 1 < parts.len() || directory;
        let mut flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        if is_directory {
            flags |= OFlags::DIRECTORY;
        }
        let file = std::fs::File::from(
            openat(&parent, *name, flags, Mode::empty()).map_err(|_| unavailable_input())?,
        );
        let metadata = file.metadata().map_err(|_| unavailable_input())?;
        let trusted_owner =
            metadata.uid() == 0 || metadata.uid() == rustix::process::geteuid().as_raw();
        let safe_sticky_ancestor = is_directory
            && index + 1 < parts.len()
            && metadata.uid() == 0
            && metadata.mode() & 0o1000 != 0;
        if !trusted_owner
            || (metadata.mode() & 0o022 != 0 && !safe_sticky_ancestor)
            || (!is_directory && !metadata.is_file())
        {
            return Err(unavailable_input());
        }
        parent = file;
    }
    Ok(parent)
}
