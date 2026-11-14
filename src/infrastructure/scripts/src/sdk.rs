//! One build-owned asset, shared by Core and Agents without a Server dependency.
use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) fn locate() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    locate_for(&executable, Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn locate_for(executable: &Path, source: &Path) -> io::Result<PathBuf> {
    // Images and standalone distributions keep assets next to their executable.
    let packaged = executable
        .parent()
        .ok_or_else(|| io::Error::other("Missing executable directory"))?
        .join("scripts/citadel.js");
    let path = if packaged.try_exists()? {
        packaged
    } else {
        // Cargo's native development/tests use the checked-in generated asset.
        source.join("generated/citadel.js")
    };
    resolve(&path)
}

pub(crate) fn resolve(path: &Path) -> io::Result<PathBuf> {
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Packaged citadel.js is missing; regenerate with cargo xtask openapi",
        ));
    }
    path.canonicalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_execution_resolves_generated_sdk_without_downloads() {
        let sdk = locate().unwrap();
        assert!(
            std::fs::read_to_string(sdk)
                .unwrap()
                .contains("export function createCitadelClient")
        );
    }

    #[test]
    fn packaged_asset_takes_precedence_and_missing_assets_fail_closed() {
        let root = std::env::temp_dir().join(format!("citadel-sdk-{}", uuid::Uuid::now_v7()));
        let packaged = root.join("scripts/citadel.js");
        std::fs::create_dir_all(packaged.parent().unwrap()).unwrap();
        std::fs::write(&packaged, "export function createCitadelClient() {}").unwrap();
        let executable = root.join("citadel-agent");
        let selected = locate_for(&executable, Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let expected = packaged.canonicalize().unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(selected, expected);
        assert_eq!(
            locate_for(&executable, &root).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
}
