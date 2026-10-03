use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=CITADEL_VERSION");
    println!("cargo:rerun-if-env-changed=CITADEL_INFORMATIONAL_VERSION");

    let version_file =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../version.json");
    println!("cargo:rerun-if-changed={}", version_file.display());

    let fallback = repository_version(&version_file);
    let informational_version = nonempty_env("CITADEL_INFORMATIONAL_VERSION")
        .or_else(|| nonempty_env("CITADEL_VERSION"))
        .unwrap_or_else(|| fallback.clone());
    let display_version = strip_build_metadata(&informational_version).to_owned();

    validate("display", &display_version);
    validate("informational", &informational_version);
    println!("cargo:rustc-env=CITADEL_BUILD_VERSION={display_version}");
    println!("cargo:rustc-env=CITADEL_BUILD_INFORMATIONAL_VERSION={informational_version}");
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
        version.len() <= 256 && !version.chars().any(char::is_control),
        "{label} version metadata is invalid"
    );
}
