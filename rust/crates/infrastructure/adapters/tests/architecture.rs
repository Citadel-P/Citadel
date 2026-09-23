use std::path::{Path, PathBuf};

fn sources(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path)
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}

#[test]
fn adapters_have_explicit_owners_without_flat_compatibility_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("src");
    let mut entries = std::fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    entries.sort();
    assert_eq!(
        entries,
        [
            "connectors",
            "external",
            "filesystem",
            "lib.rs",
            "persistence",
            "security"
        ]
    );
    assert!(
        !root.join("queries").exists(),
        "SQL belongs beside its consumer"
    );
    let facade = std::fs::read_to_string(source.join("lib.rs")).unwrap();
    for line in facade
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        assert!(
            line == "#![forbid(unsafe_code)]"
                || (line.starts_with("pub mod ") && line.ends_with(';')),
            "adapter root must only declare its groups: {line}"
        );
    }
}

#[test]
fn concrete_persistence_and_security_implementations_stay_separate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for path in sources(&root) {
        let source = std::fs::read_to_string(&path).unwrap();
        assert!(
            !source.contains("#[path"),
            "{} bypasses normal module paths",
            path.display()
        );
        if !path.starts_with(root.join("persistence")) {
            for line in source.lines().map(str::trim) {
                assert!(
                    !line.starts_with("pub struct Postgres"),
                    "{} declares a PostgreSQL implementation outside persistence",
                    path.display()
                );
            }
        }
        if path.starts_with(root.join("security")) {
            for forbidden in [
                "sqlx::",
                "PgPool",
                "PostgresLicenseStore",
                "PostgresMfaStore",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} mixes security with {forbidden}",
                    path.display()
                );
            }
        }
    }
}
