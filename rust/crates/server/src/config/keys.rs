//! Persist generated keys atomically in the data volume.
use super::{ConfigError, STANDARD};
use base64::Engine;
use std::{fs, io::Write, path::Path};
use zeroize::Zeroizing;

pub(super) fn load_or_create(
    root: &Path,
    file: &str,
    name: &'static str,
) -> Result<Zeroizing<String>, ConfigError> {
    let error = |e: std::io::Error| ConfigError::Invalid {
        name,
        message: format!("cannot access persisted key: {e}"),
    };
    let target = root.join(file);
    match fs::read_to_string(&target) {
        Ok(value) => return Ok(Zeroizing::new(value)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(error(e)),
    }
    fs::create_dir_all(root).map_err(error)?;
    let temporary = root.join(format!(".{file}-{}", uuid::Uuid::now_v7()));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(error)?;
        let mut bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(bytes.as_mut()).map_err(|_| ConfigError::Invalid {
            name,
            message: "cannot generate a cryptographic key".into(),
        })?;
        let value = Zeroizing::new(STANDARD.encode(bytes.as_ref()));
        file.write_all(value.as_bytes()).map_err(error)?;
        file.sync_all().map_err(error)?;
        // A concurrent process can win. Never overwrite its committed key.
        match fs::hard_link(&temporary, &target) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(error(e)),
        }
        fs::read_to_string(&target)
            .map(Zeroizing::new)
            .map_err(error)
    })();
    let _ = fs::remove_file(temporary);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_keys_are_private_stable_and_shared_by_concurrent_starters() {
        let root = std::env::temp_dir().join(format!("citadel-keys-{}", uuid::Uuid::now_v7()));
        let values = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| scope.spawn(|| load_or_create(&root, "jwtsecret", "Jwt__Key").unwrap()))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(values.iter().all(|v| v.as_str() == values[0].as_str()));
        assert_eq!(STANDARD.decode(values[0].as_bytes()).unwrap().len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.join("jwtsecret"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::write(root.join("jwtsecret"), "existing-key").unwrap();
        assert_eq!(
            load_or_create(&root, "jwtsecret", "Jwt__Key")
                .unwrap()
                .as_str(),
            "existing-key"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
