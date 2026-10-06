use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CITADEL_VERSION");
    println!("cargo:rerun-if-env-changed=CITADEL_INFORMATIONAL_VERSION");

    let version_file =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../version.json");
    println!("cargo:rerun-if-changed={}", version_file.display());

    let (display_version, informational_version) = versions(
        &repository_version(&version_file),
        nonempty_env("CITADEL_VERSION"),
        nonempty_env("CITADEL_INFORMATIONAL_VERSION"),
    );

    validate("display", &display_version);
    validate("informational", &informational_version);
    println!("cargo:rustc-env=CITADEL_BUILD_VERSION={display_version}");
    println!("cargo:rustc-env=CITADEL_BUILD_INFORMATIONAL_VERSION={informational_version}");
}

fn versions(
    product: &str,
    display: Option<String>,
    informational: Option<String>,
) -> (String, String) {
    match (display, informational) {
        (Some(display), Some(informational)) => (display, informational),
        (Some(display), None) => (display.clone(), display),
        (None, Some(informational)) => (strip_build_metadata(&informational).into(), informational),
        (None, None) => (
            format!("{product}-dev.unknown"),
            format!("{product}-dev.unknown+source.unknown"),
        ),
    }
}

fn repository_version(path: &PathBuf) -> String {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    let document: serde_json::Value = serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("could not parse {}: {error}", path.display()));
    document["version"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{} has no non-empty version", path.display()))
        .to_owned()
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn strip_build_metadata(version: &str) -> &str {
    version
        .split_once('+')
        .map_or(version, |(display, _)| display)
}

fn validate(label: &str, version: &str) {
    assert!(
        version.len() <= 256 && valid_semver(version),
        "{label} version metadata is invalid"
    );
}

fn valid_semver(version: &str) -> bool {
    let identifiers = |text: &str, prerelease: bool| {
        text.split('.').all(|part| {
            !part.is_empty()
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !(prerelease
                    && part.len() > 1
                    && part.starts_with('0')
                    && part.bytes().all(|b| b.is_ascii_digit()))
        })
    };
    let (version, metadata) = version
        .split_once('+')
        .map_or((version, None), |(v, m)| (v, Some(m)));
    if metadata.is_some_and(|m| !identifiers(m, false)) {
        return false;
    }
    let (base, pre) = version
        .split_once('-')
        .map_or((version, None), |(v, p)| (v, Some(p)));
    let parts: Vec<_> = base.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && (p.len() == 1 || !p.starts_with('0'))
        })
        && pre.is_none_or(|p| identifiers(p, true))
}
