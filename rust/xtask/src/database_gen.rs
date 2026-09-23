use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SOURCE_KIND: &str = "dotnet-development-baseline-evidence";
const SOURCE_PATH: &str = "src/Citadel.Infrastructure/Scripts/script0001.sql";
const IMPORTED_TABLES: usize = 82;
const EXPECTED_SEED_INSERTS: usize = 92;
const EF_HEADER: &str = r#"CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    "MigrationId" character varying(150) NOT NULL,
    "ProductVersion" character varying(32) NOT NULL,
    CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY ("MigrationId")
);

START TRANSACTION;
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptedContracts {
    files: Vec<AcceptedFile>,
}

#[derive(Deserialize)]
struct AcceptedFile {
    kind: String,
    path: String,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationManifest<'a> {
    schema_version: u32,
    source: MigrationSource<'a>,
    schema: SchemaEntry<'a>,
    migrations: Vec<MigrationEntry<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationSource<'a> {
    path: &'a str,
    sha256: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationEntry<'a> {
    id: &'a str,
    name: &'a str,
    file: &'a str,
    sha256: &'a str,
    transactional: bool,
    generated_by: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemaEntry<'a> {
    file: &'a str,
    sha256: &'a str,
}

pub fn import_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let database_root = rust_root.join("crates/infrastructure/database");
    if database_root.join("generated").exists()
        && migration_files(&database_root.join("generated"))
            .is_ok_and(|migrations| migrations.len() > 1)
    {
        return Err("the Rust database baseline already has follow-up migrations".into());
    }
    let repository_root = rust_root
        .parent()
        .expect("the Rust workspace must be inside the Citadel repository");
    let accepted_path = rust_root.join("phase1/accepted-contracts.json");
    let accepted: AcceptedContracts = serde_json::from_slice(&fs::read(&accepted_path)?)?;
    let source = accepted
        .files
        .iter()
        .find(|file| file.kind == SOURCE_KIND && file.path == SOURCE_PATH)
        .ok_or("accepted database baseline contract is missing")?;
    let source_path = repository_root.join(&source.path);
    let source_bytes = fs::read(&source_path)?;
    let source_hash = digest(&source_bytes);
    if source_hash != source.sha256 {
        return Err(format!(
            "database baseline checksum mismatch for {}: expected {}, got {}",
            source_path.display(),
            source.sha256,
            source_hash
        )
        .into());
    }

    let source_sql = String::from_utf8(source_bytes)?;
    let source_sql = source_sql
        .strip_prefix('\u{feff}')
        .unwrap_or(&source_sql)
        .replace("\r\n", "\n");
    let product_sql = rust_baseline(&source_sql)?;
    let schema = format!(
        "-- Citadel Rust declarative schema authority.\n\
         -- Imported once from: {SOURCE_PATH}\n\n{product_sql}"
    );
    let migration = format!(
        "-- @generated pre-release baseline; do not edit.\n\
         -- Imported once from: {SOURCE_PATH}\n\n{product_sql}"
    );
    validate_baseline(&schema, IMPORTED_TABLES)?;
    let schema_hash = digest(schema.as_bytes());
    let migration_hash = digest(migration.as_bytes());
    let manifest = MigrationManifest {
        schema_version: 1,
        source: MigrationSource {
            path: SOURCE_PATH,
            sha256: &source.sha256,
        },
        schema: SchemaEntry {
            file: "src/schema/schema.sql",
            sha256: &schema_hash,
        },
        migrations: vec![MigrationEntry {
            id: "0001",
            name: "initial",
            file: "0001_initial.sql",
            sha256: &migration_hash,
            transactional: true,
            generated_by: "citadel-xtask-v1",
        }],
    };
    let manifest = format!("{}\n", serde_json::to_string_pretty(&manifest)?);

    write_generated(
        &database_root.join("src/schema/schema.sql"),
        schema.as_bytes(),
    )?;
    write_generated(
        &database_root.join("generated/0001_initial.sql"),
        migration.as_bytes(),
    )?;
    write_generated(
        &database_root.join("generated/migrations.json"),
        manifest.as_bytes(),
    )?;
    refresh_catalog()?;

    println!(
        "generated database baseline ({} tables, {} seed inserts, SHA-256 {})",
        IMPORTED_TABLES, EXPECTED_SEED_INSERTS, migration_hash
    );
    Ok(())
}

pub fn refresh_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let database_root = rust_root.join("crates/infrastructure/database");
    let generated_root = database_root.join("generated");
    let migrations = migration_files(&generated_root)?;
    if migrations.len() != 1
        || migrations[0].file_name().and_then(|value| value.to_str()) != Some("0001_initial.sql")
    {
        return Err("the unreleased Rust database must contain only 0001_initial.sql".into());
    }

    let schema = fs::read(database_root.join("src/schema/schema.sql"))?;
    let product_sql = std::str::from_utf8(product_body(&schema)?)?;
    let table_count = product_sql.matches("CREATE TABLE ").count();
    validate_baseline(product_sql, table_count)?;
    let migration = format!(
        "-- @generated pre-release baseline; do not edit.\n\
         -- Generated from: crates/infrastructure/database/src/schema/schema.sql\n\n{product_sql}"
    );
    write_generated(
        &generated_root.join("0001_initial.sql"),
        migration.as_bytes(),
    )?;
    refresh_catalog()?;

    println!(
        "refreshed unreleased database baseline ({table_count} tables, SHA-256 {})",
        digest(migration.as_bytes())
    );
    Ok(())
}

pub fn refresh_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let database_root = rust_root.join("crates/infrastructure/database");
    let generated_root = database_root.join("generated");
    let existing: serde_json::Value =
        serde_json::from_slice(&fs::read(generated_root.join("migrations.json"))?)?;
    let source_path = existing
        .pointer("/source/path")
        .and_then(serde_json::Value::as_str)
        .ok_or("database manifest source path is missing")?;
    let source_hash = existing
        .pointer("/source/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("database manifest source checksum is missing")?;

    let schema_path = database_root.join("src/schema/schema.sql");
    let schema_hash = digest(&fs::read(&schema_path)?);
    let migration_files = migration_files(&generated_root)?;
    if migration_files.len() != 1
        || migration_files[0]
            .file_name()
            .and_then(|value| value.to_str())
            != Some("0001_initial.sql")
    {
        return Err("the unreleased Rust database must contain only 0001_initial.sql".into());
    }
    let mut migrations = Vec::with_capacity(migration_files.len());
    let mut owned = Vec::with_capacity(migration_files.len());
    for (expected, path) in migration_files.iter().enumerate() {
        let file = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("migration file name is not valid UTF-8")?;
        let stem = file
            .strip_suffix(".sql")
            .ok_or("migration file does not end with .sql")?;
        let (id, name) = stem
            .split_once('_')
            .ok_or("migration file must be named NNNN_name.sql")?;
        let numeric = id.parse::<usize>()?;
        if id.len() != 4 || numeric != expected + 1 || name.is_empty() {
            return Err(format!(
                "migration '{file}' is not part of a contiguous NNNN_name.sql catalog"
            )
            .into());
        }
        let bytes = fs::read(path)?;
        owned.push((
            id.to_owned(),
            name.to_owned(),
            file.to_owned(),
            digest(&bytes),
            true,
            "citadel-xtask-v1".to_owned(),
        ));
    }
    for (id, name, file, hash, transactional, generated_by) in &owned {
        migrations.push(MigrationEntry {
            id,
            name,
            file,
            sha256: hash,
            transactional: *transactional,
            generated_by,
        });
    }
    let schema_version = u32::try_from(migrations.len())?;
    let manifest = MigrationManifest {
        schema_version,
        source: MigrationSource {
            path: source_path,
            sha256: source_hash,
        },
        schema: SchemaEntry {
            file: "src/schema/schema.sql",
            sha256: &schema_hash,
        },
        migrations,
    };
    write_generated(
        &generated_root.join("migrations.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?).as_bytes(),
    )?;
    write_generated(
        &generated_root.join("catalog.rs"),
        migration_catalog_source(&owned).as_bytes(),
    )?;
    println!("refreshed database catalog at schema version {schema_version}");
    Ok(())
}

fn migration_catalog_source(
    migrations: &[(String, String, String, String, bool, String)],
) -> String {
    let mut source = String::from(
        "// @generated by `cargo run -p xtask -- database refresh-catalog`; do not edit.\n\
         const MIGRATIONS: &[EmbeddedMigration] = &[\n",
    );
    for (id, name, file, _, transactional, _) in migrations {
        source.push_str(&format!(
            "    EmbeddedMigration {{ id: \"{id}\", name: \"{name}\", file: \"{file}\", sql: include_str!(\"{file}\"), transactional: {transactional} }},\n"
        ));
    }
    source.push_str("];\n");
    source
}

fn migration_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("sql")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.as_bytes().first().is_some_and(u8::is_ascii_digit))
        })
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() {
        return Err("database migration catalog is empty".into());
    }
    Ok(paths)
}

pub fn verify() -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let database_root = rust_root.join("crates/infrastructure/database");
    let schema = fs::read(database_root.join("src/schema/schema.sql"))?;
    let migration = fs::read(database_root.join("generated/0001_initial.sql"))?;
    if product_body(&schema)? != product_body(&migration)? {
        return Err("Rust schema authority and unreleased baseline product SQL differ".into());
    }
    let migration_sql = std::str::from_utf8(&migration)?;
    let expected_tables = std::str::from_utf8(&schema)?
        .matches("CREATE TABLE ")
        .count();
    validate_baseline(migration_sql, expected_tables)?;
    let migration_hash = digest(&migration);
    let schema_hash = digest(&schema);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(database_root.join("generated/migrations.json"))?)?;
    let migrations = manifest
        .get("migrations")
        .and_then(serde_json::Value::as_array)
        .ok_or("database manifest has no migration catalog")?;
    let migration_files = migration_files(&database_root.join("generated"))?;
    if manifest
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
        || migrations.len() != 1
        || migration_files.len() != 1
        || manifest
            .get("schema")
            .and_then(|schema| schema.get("file"))
            .and_then(serde_json::Value::as_str)
            != Some("src/schema/schema.sql")
        || manifest
            .get("schema")
            .and_then(|schema| schema.get("sha256"))
            .and_then(serde_json::Value::as_str)
            != Some(schema_hash.as_str())
    {
        return Err("database manifest does not match the Rust schema catalog".into());
    }
    for (index, (entry, path)) in migrations.iter().zip(&migration_files).enumerate() {
        let file = path.file_name().and_then(|value| value.to_str());
        let expected_id = format!("{:04}", index + 1);
        let actual_hash = digest(&fs::read(path)?);
        if entry.get("id").and_then(serde_json::Value::as_str) != Some(expected_id.as_str())
            || entry.get("file").and_then(serde_json::Value::as_str) != file
            || entry.get("sha256").and_then(serde_json::Value::as_str) != Some(actual_hash.as_str())
        {
            return Err(format!("database manifest entry {expected_id} is stale").into());
        }
        if entry
            .get("transactional")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            return Err(format!("database migration {expected_id} must be transactional").into());
        }
    }

    let catalog_entries = migrations
        .iter()
        .map(|entry| {
            Ok((
                entry["id"]
                    .as_str()
                    .ok_or("migration id is missing")?
                    .to_owned(),
                entry["name"]
                    .as_str()
                    .ok_or("migration name is missing")?
                    .to_owned(),
                entry["file"]
                    .as_str()
                    .ok_or("migration file is missing")?
                    .to_owned(),
                entry["sha256"]
                    .as_str()
                    .ok_or("migration checksum is missing")?
                    .to_owned(),
                entry["transactional"]
                    .as_bool()
                    .ok_or("migration transaction metadata is missing")?,
                entry["generatedBy"]
                    .as_str()
                    .ok_or("migration generator metadata is missing")?
                    .to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let catalog_path = database_root.join("generated/catalog.rs");
    if fs::read_to_string(&catalog_path)? != migration_catalog_source(&catalog_entries) {
        return Err("embedded database migration catalog is stale".into());
    }

    println!(
        "verified database schema version {} (baseline SHA-256 {})",
        migrations.len(),
        migration_hash
    );
    Ok(())
}

fn rust_baseline(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let without_header = source
        .strip_prefix(EF_HEADER)
        .ok_or("database baseline has an unexpected EF history header")?;
    let history_start = without_header
        .rfind("INSERT INTO \"__EFMigrationsHistory\"")
        .ok_or("database baseline has no final EF history insert")?;
    let product_sql = without_header[..history_start].trim_end();
    let trailer = &without_header[history_start..];
    if !trailer.trim_end().ends_with("COMMIT;") || trailer.matches("INSERT INTO").count() != 1 {
        return Err("database baseline has an unexpected EF history trailer".into());
    }

    Ok(format!("{product_sql}\n"))
}

fn product_body(contents: &[u8]) -> Result<&[u8], Box<dyn std::error::Error>> {
    contents
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| &contents[index + 2..])
        .ok_or_else(|| "database artifact has no provenance header".into())
}

fn validate_baseline(sql: &str, expected_tables: usize) -> Result<(), Box<dyn std::error::Error>> {
    if sql.contains("__EFMigrationsHistory")
        || sql.contains("START TRANSACTION")
        || sql.contains("COMMIT;")
    {
        return Err("Rust baseline retained EF or outer transaction metadata".into());
    }
    let tables = sql.matches("CREATE TABLE ").count();
    let inserts = sql.matches("INSERT INTO ").count();
    if tables != expected_tables || inserts != EXPECTED_SEED_INSERTS {
        return Err(format!(
            "Rust baseline has {tables} tables and {inserts} seed inserts; expected {expected_tables} and {EXPECTED_SEED_INSERTS}"
        )
        .into());
    }
    Ok(())
}

fn write_generated(path: &Path, contents: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_to_strip_an_unknown_baseline_shape() {
        assert!(rust_baseline("CREATE TABLE users (id uuid);\n").is_err());
    }

    #[test]
    fn generates_a_static_embedded_catalog_without_runtime_discovery() {
        let entries = vec![(
            "0001".to_owned(),
            "initial".to_owned(),
            "0001_initial.sql".to_owned(),
            "checksum".to_owned(),
            true,
            "generator".to_owned(),
        )];
        let source = migration_catalog_source(&entries);
        assert!(source.contains("include_str!(\"0001_initial.sql\")"));
        assert!(source.contains("transactional: true"));
    }
}
