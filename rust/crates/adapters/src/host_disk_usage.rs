//! Read the filesystem containing Docker's data root, as in the .NET provider.
use citadel_platforms::HostDiskUsage;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU8, Ordering},
};

pub struct HostDiskUsageProvider {
    root: PathBuf,
    availability: AtomicU8,
}

impl HostDiskUsageProvider {
    pub fn from_env() -> Self {
        Self::new(
            std::env::var_os("CITADEL_HOST_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| "/host".into()),
        )
    }

    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            availability: AtomicU8::new(0),
        }
    }

    pub fn read(&self, docker_root: &str) -> Option<HostDiskUsage> {
        let usage = self.read_inner(docker_root);
        let available = if usage.is_some() { 1 } else { 2 };
        if self.availability.swap(available, Ordering::Relaxed) != available {
            if usage.is_some() {
                tracing::info!(host_root = %self.root.display(), docker_root, "Host disk metrics available");
            } else {
                tracing::warn!(host_root = %self.root.display(), docker_root, "Host disk metrics unavailable: the Docker data filesystem must be accessible under CITADEL_HOST_ROOT (default /host)");
            }
        }
        usage
    }

    #[cfg(target_os = "linux")]
    fn read_inner(&self, docker_root: &str) -> Option<HostDiskUsage> {
        let path = resolve_host_path(&self.root, docker_root)?;
        // Do not follow a symlink out of the configured host view.
        let root = self.root.canonicalize().ok()?;
        let path = path.canonicalize().ok()?;
        if !path.starts_with(root) {
            return None;
        }
        let stat = rustix::fs::statvfs(path).ok()?;
        calculate_usage(stat.f_frsize, stat.f_bsize, stat.f_blocks, stat.f_bfree)
    }

    #[cfg(not(target_os = "linux"))]
    fn read_inner(&self, _docker_root: &str) -> Option<HostDiskUsage> {
        None
    }
}

fn resolve_host_path(root: &Path, docker_root: &str) -> Option<PathBuf> {
    if !root.is_absolute() || root.as_os_str().as_encoded_bytes().contains(&0) {
        return None;
    }
    if docker_root.trim().is_empty() {
        return Some(root.to_owned());
    }
    if !docker_root.starts_with('/')
        || docker_root.contains(['\0', '\\'])
        || docker_root.split('/').any(|p| p == "." || p == "..")
    {
        return None;
    }
    Some(root.join(docker_root.trim_start_matches('/')))
}

fn calculate_usage(
    fragment_size: u64,
    block_size: u64,
    blocks: u64,
    free: u64,
) -> Option<HostDiskUsage> {
    let size = if fragment_size == 0 {
        block_size
    } else {
        fragment_size
    };
    let total = blocks.checked_mul(size)?;
    let used = total.checked_sub(free.checked_mul(size)?)?;
    HostDiskUsage::new(
        i64::try_from(used).ok()?,
        i64::try_from(total).ok()?,
        used as f64 / total as f64 * 100.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_paths_support_container_and_native_roots_without_traversal() {
        assert_eq!(
            resolve_host_path(Path::new("/host"), "/var/lib/docker").unwrap(),
            Path::new("/host/var/lib/docker")
        );
        assert_eq!(
            resolve_host_path(Path::new("/"), "/var/lib/docker").unwrap(),
            Path::new("/var/lib/docker")
        );
        for path in [
            "var/lib/docker",
            "/../docker",
            "/var/./docker",
            "/a\\b",
            "/a\0b",
        ] {
            assert!(resolve_host_path(Path::new("/host"), path).is_none());
        }
        assert!(resolve_host_path(Path::new("host"), "/var/lib/docker").is_none());
    }
    #[test]
    fn uses_free_blocks_including_reserved_space_and_rejects_invalid_capacity() {
        let disk = calculate_usage(4096, 1024, 100, 10).unwrap();
        assert_eq!(disk.total_bytes, 409600);
        assert_eq!(disk.used_bytes, 368640);
        assert_eq!(disk.usage_percent, 90.0);
        assert_eq!(calculate_usage(0, 4096, 100, 10), Some(disk));
        assert_eq!(calculate_usage(1, 1, 100, 100).unwrap().usage_percent, 0.0);
        for args in [
            (0, 0, 1, 0),
            (1, 1, 0, 0),
            (1, 1, 10, 11),
            (u64::MAX, 1, 2, 0),
            (1, 1, u64::MAX, 0),
        ] {
            assert!(calculate_usage(args.0, args.1, args.2, args.3).is_none());
        }
    }
    #[test]
    #[ignore = "requires the Docker host root mounted at CITADEL_HOST_ROOT and CITADEL_TEST_DOCKER_ROOT"]
    fn probes_configured_docker_host_mount() {
        let docker_root = std::env::var("CITADEL_TEST_DOCKER_ROOT").unwrap();
        let disk = HostDiskUsageProvider::from_env()
            .read(&docker_root)
            .expect("Docker host filesystem must be measurable");
        assert!(disk.total_bytes > 0 && disk.used_bytes <= disk.total_bytes);
        println!(
            "Docker host disk: {} / {} bytes ({:.2}%)",
            disk.used_bytes, disk.total_bytes, disk.usage_percent
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn probes_real_filesystem_and_keeps_missing_mount_unavailable() {
        assert!(
            HostDiskUsageProvider::new("/".into())
                .read("/tmp")
                .is_some()
        );
        assert!(
            HostDiskUsageProvider::new("/missing-citadel-host-mount".into())
                .read("/var/lib/docker")
                .is_none()
        );
    }
}
