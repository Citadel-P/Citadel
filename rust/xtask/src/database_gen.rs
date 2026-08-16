use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SOURCE_KIND: &str = "dotnet-development-baseline-evidence";
const SOURCE_PATH: &str = "src/Citadel.Infrastructure/Scripts/script0001.sql";
const EXPECTED_TABLES: usize = 82;
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
    migrations: [MigrationEntry<'a>; 1],
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
    let database_root = rust_root.join("crates/database");
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
        "-- @generated immutable migration; do not edit.\n\
         -- Imported once from: {SOURCE_PATH}\n\n{product_sql}"
    );
    validate_baseline(&schema)?;
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
        migrations: [MigrationEntry {
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

    println!(
        "generated database baseline ({} tables, {} seed inserts, SHA-256 {})",
        EXPECTED_TABLES, EXPECTED_SEED_INSERTS, migration_hash
    );
    Ok(())
}

pub fn verify() -> Result<(), Box<dyn std::error::Error>> {
    let rust_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the Rust workspace");
    let database_root = rust_root.join("crates/database");
    let schema = fs::read(database_root.join("src/schema/schema.sql"))?;
    let migration = fs::read(database_root.join("generated/0001_initial.sql"))?;
    if product_body(&schema)? != product_body(&migration)? {
        return Err("Rust schema authority and initial migration product SQL differ".into());
    }
    let sql = std::str::from_utf8(&schema)?;
    validate_baseline(sql)?;
    let migration_hash = digest(&migration);
    let schema_hash = digest(&schema);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(database_root.join("generated/migrations.json"))?)?;
    let migrations = manifest
        .get("migrations")
        .and_then(serde_json::Value::as_array)
        .ok_or("database manifest has no migration catalog")?;
    if manifest
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
        || migrations.len() != 1
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
        || migrations[0].get("id").and_then(serde_json::Value::as_str) != Some("0001")
        || migrations[0]
            .get("name")
            .and_then(serde_json::Value::as_str)
            != Some("initial")
        || migrations[0]
            .get("file")
            .and_then(serde_json::Value::as_str)
            != Some("0001_initial.sql")
        || migrations[0]
            .get("transactional")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || migrations[0]
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            != Some(migration_hash.as_str())
    {
        return Err("database manifest does not match the immutable Rust baseline".into());
    }

    println!(
        "verified database baseline ({} tables, {} seed inserts, SHA-256 {})",
        EXPECTED_TABLES, EXPECTED_SEED_INSERTS, migration_hash
    );
    Ok(())
}

fn product_body(contents: &[u8]) -> Result<&[u8], Box<dyn std::error::Error>> {
    contents
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| &contents[index + 2..])
        .ok_or_else(|| "database artifact has no provenance header".into())
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

fn validate_baseline(sql: &str) -> Result<(), Box<dyn std::error::Error>> {
    if sql.contains("__EFMigrationsHistory")
        || sql.contains("START TRANSACTION")
        || sql.contains("COMMIT;")
    {
        return Err("Rust baseline retained EF or outer transaction metadata".into());
    }
    let tables = sql.matches("CREATE TABLE ").count();
    let inserts = sql.matches("INSERT INTO ").count();
    if tables != EXPECTED_TABLES || inserts != EXPECTED_SEED_INSERTS {
        return Err(format!(
            "Rust baseline has {tables} tables and {inserts} seed inserts; expected {EXPECTED_TABLES} and {EXPECTED_SEED_INSERTS}"
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
}
