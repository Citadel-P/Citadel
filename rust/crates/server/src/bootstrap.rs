//! Startup-only secret-file bootstrap. Reuses the interactive setup transaction;
//! it neither issues a browser session nor reopens a completed installation.
use std::path::Path;

use citadel_identity::{IdentityError, IdentityService};

use tokio::io::AsyncReadExt;

use zeroize::Zeroizing;

pub async fn initialize_from_environment(identity: &IdentityService) -> Result<(), IdentityError> {
    // This check must precede configuration/file access: the operator can remove
    // the mounted password after setup without breaking subsequent restarts.
    if !identity.setup_status().await?.requires_setup {
        return Ok(());
    }
    let settings = [
        "Bootstrap__AdminName",
        "Bootstrap__AdminEmail",
        "Bootstrap__AdminPasswordFile",
    ];
    let values = settings.map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()));
    if values.iter().all(Option::is_none) {
        return Ok(());
    }
    for (key, value) in settings.iter().zip(&values) {
        if value.is_none() {
            return Err(IdentityError::Validation(format!(
                "Incomplete bootstrap administrator configuration. Missing: {key}."
            )));
        }
    }
    let [Some(name), Some(email), Some(path)] = values else {
        unreachable!()
    };
    let password = read_password(Path::new(&path)).await?;
    match identity
        .initialize_user_in_mode(
            citadel_identity::InitializeCitadel {
                name,
                email,
                password,
            },
            citadel_identity::SetupInitializationMode::Unattended,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("Citadel initial administrator created by unattended bootstrap");
            Ok(())
        }
        // Another Core can win the same locked setup transaction during startup.
        Err(IdentityError::SetupAlreadyComplete) => Ok(()),
        Err(error) => Err(error),
    }
}

async fn read_password(path: &Path) -> Result<String, IdentityError> {
    if !path.is_absolute() {
        return Err(invalid("must be an absolute path"));
    }
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| invalid("must reference a readable regular file"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("must reference a readable regular file"));
    }
    if metadata.len() > 1024 {
        return Err(invalid("exceeds the 1 KiB limit"));
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| invalid("must reference a readable regular file"))?;
    if !file
        .metadata()
        .await
        .map_err(|_| invalid("cannot read metadata"))?
        .is_file()
    {
        return Err(invalid("must reference a readable regular file"));
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(1025));
    file.take(1025)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| invalid("could not be read"))?;
    decode_password(&bytes)
}

fn decode_password(bytes: &[u8]) -> Result<String, IdentityError> {
    if bytes.len() > 1024 {
        return Err(invalid("exceeds the 1 KiB limit"));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("must contain valid UTF-8 text"))?;
    let password = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if password.contains(['\r', '\n']) {
        return Err(invalid("must contain a single password line"));
    }
    Ok(password.to_owned())
}

fn invalid(message: &str) -> IdentityError {
    IdentityError::Validation(format!("Bootstrap__AdminPasswordFile {message}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    // InitialAdministratorBootstrapServiceTests.ReadPasswordAsync_ShouldReadUtf8PasswordAndTrimOneTrailingNewline.
    #[test]
    fn password_file_accepts_utf8_bom_and_one_newline_without_trimming_spaces() {
        for ending in ["", "\n", "\r\n"] {
            assert_eq!(
                decode_password(format!("\u{feff}  pässphrase  {ending}").as_bytes()).unwrap(),
                "  pässphrase  "
            );
        }
    }

    // InitialAdministratorBootstrapServiceTests.ReadPasswordAsync_ShouldRejectContentLargerThanOneKiB.
    #[test]
    fn password_file_rejects_oversized_invalid_utf8_and_multiple_lines_without_echoing_values() {
        for content in [
            vec![b'x'; 1025],
            vec![0xff],
            b"first-secret\nsecond-secret".to_vec(),
            b"secret\n\n".to_vec(),
            b"secret\r".to_vec(),
        ] {
            let error = decode_password(&content).unwrap_err().to_string();
            assert!(!error.contains("first-secret"));
            assert!(!error.contains("second-secret"));
        }
    }

    // InitialAdministratorBootstrapServiceTests.ReadPasswordAsync_ShouldRejectDirectories.
    #[tokio::test]
    async fn password_file_rejects_directories_missing_files_and_relative_paths() {
        for path in [
            std::env::temp_dir(),
            std::env::temp_dir().join(uuid::Uuid::now_v7().to_string()),
            "relative-password".into(),
        ] {
            assert!(read_password(&path).await.is_err());
        }
    }
}
