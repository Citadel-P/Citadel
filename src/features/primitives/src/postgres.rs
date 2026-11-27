//! PostgreSQL version evidence for Citadel's control-plane database.
pub const SUPPORTED_MAJOR: u32 = 19;

pub fn require_supported(version: u32) -> Result<u32, String> {
    let major = version / 10_000;
    if major == SUPPORTED_MAJOR {
        Ok(version)
    } else {
        Err(format!(
            "Citadel requires PostgreSQL {SUPPORTED_MAJOR}.x. Observed PostgreSQL major {major}. Automatic major-version upgrades are unsupported."
        ))
    }
}

/// Released PostgreSQL tools may append their distribution's package revision.
pub fn release_number(value: &str) -> Option<u32> {
    let value = match value.split_once(" (") {
        Some((version, package))
            if package.ends_with(')')
                && !package[..package.len() - 1].is_empty()
                && !package[..package.len() - 1].contains(['(', ')', '\n', '\r']) =>
        {
            version
        }
        Some(_) => return None,
        None => value,
    };
    let (major, minor) = value.split_once('.')?;
    let number = |s: &str| {
        if s.is_empty()
            || !s.bytes().all(|c| c.is_ascii_digit())
            || (s.len() > 1 && s.starts_with('0'))
        {
            return None;
        }
        s.parse::<u32>().ok()
    };
    let minor = number(minor)?;
    (minor < 10_000).then_some(())?;
    number(major)?.checked_mul(10_000)?.checked_add(minor)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_a_released_version_of_the_supported_major() {
        for bad in [
            "",
            "19",
            "19beta1",
            "19.0rc1",
            "19.0.1",
            "19.10000",
            "19.0 20.0",
            "19.0\n18.1",
        ] {
            assert_eq!(release_number(bad), None);
        }
        for version in [0, 90624, 180007, 200000] {
            assert!(require_supported(version).is_err());
        }
        assert_eq!(
            require_supported(release_number("19.0 (Debian 19.0-1)").unwrap()),
            Ok(190000)
        );
        assert!(release_number("19.9").unwrap() < release_number("19.10").unwrap());
    }
}
