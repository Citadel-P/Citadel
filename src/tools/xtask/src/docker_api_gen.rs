//! Reproducible Docker protocol generation from the pinned schema and templates.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Source {
    schema_path: String,
    schema_sha256: String,
    api_version: String,
    generator_version: String,
    generator_url: String,
    generator_sha256: String,
}

#[derive(Deserialize)]
struct Patch {
    path: String,
    #[serde(default)]
    expected: Option<Value>,
    value: Value,
}

pub fn generate(check: bool) -> Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap();
    let crate_dir = workspace.join("src/infrastructure/docker-api");
    let codegen = crate_dir.join("codegen");
    let source: Source = serde_json::from_slice(&fs::read(codegen.join("source.json"))?)?;
    let bytes = fs::read(workspace.join(&source.schema_path))?;
    verify_hash(&bytes, &source.schema_sha256, "Docker schema")?;
    let mut schema: Value = serde_yaml_ng::from_slice(&bytes)?;
    if schema["info"]["version"] != source.api_version {
        return Err("Docker schema API version differs from source.json".into());
    }
    let patches: Vec<Patch> =
        serde_json::from_slice(&fs::read(codegen.join("patches/wire.json"))?)?;
    apply_patches(&mut schema, patches)?;

    let jar = generator_jar(workspace, &source)?;
    // Keep scratch output outside the source tree. RAII cleanup also covers failures.
    let temp = Scratch(workspace.join(format!("target/docker-api-codegen-{}", std::process::id())));
    fs::create_dir(&temp.0)?;
    let input = temp.0.join("schema.json");
    fs::write(&input, serde_json::to_vec_pretty(&schema)?)?;
    let output = temp.0.join("output");
    let java = std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/java"))
        .unwrap_or_else(|| "java".into());
    let result = Command::new(java)
        .args([
            "-Dfile.encoding=UTF-8",
            "-Duser.language=en",
            "-Duser.country=US",
            "-jar",
        ])
        .arg(jar)
        .args(["generate", "-g", "rust", "-i"])
        .arg(input)
        .arg("-c")
        .arg(codegen.join("generator.json"))
        .arg("-t")
        .arg(codegen.join("templates"))
        .arg("-o")
        .arg(&output)
        .args([
            "--global-property",
            "apiDocs=false,modelDocs=false,apiTests=false,modelTests=false",
        ])
        .output()
        .map_err(|error| format!("Cannot run Java (install Java 17+ or set JAVA_HOME): {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "OpenAPI Generator failed:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    // rustfmt is a deterministic generation step, using the pinned workspace toolchain.
    let status = Command::new("rustfmt")
        .current_dir(workspace)
        .args(["--edition", "2024"])
        .arg(output.join("src/lib.rs"))
        .status()?;
    if !status.success() {
        return Err("rustfmt failed on generated Docker source".into());
    }
    let expected = generated_files(&output)?;
    if check {
        let actual = generated_files(&crate_dir)?;
        let differences: Vec<_> = expected
            .keys()
            .chain(actual.keys())
            .filter(|path| expected.get(*path) != actual.get(*path))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if !differences.is_empty() {
            return Err(format!(
                "Generated Docker API differs at {differences:?}; run cargo xtask docker-api"
            )
            .into());
        }
    } else {
        // Only src and Cargo.toml are generated. Never touch tests/codegen/docs.
        if crate_dir.join("src").exists() {
            fs::remove_dir_all(crate_dir.join("src"))?;
        }
        for (path, bytes) in &expected {
            let destination = crate_dir.join(path);
            fs::create_dir_all(destination.parent().unwrap())?;
            fs::write(destination, bytes)?;
        }
    }
    println!(
        "Docker API {}: {} generated files verified with OpenAPI Generator {}",
        source.api_version,
        expected.len(),
        source.generator_version
    );
    Ok(())
}

fn generator_jar(workspace: &Path, source: &Source) -> Result<PathBuf> {
    let jar = std::env::var_os("CITADEL_OPENAPI_GENERATOR_JAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace.join(format!(
                "target/codegen/openapi-generator-cli-{}.jar",
                source.generator_version
            ))
        });
    if !jar.exists() {
        if std::env::var_os("CITADEL_OPENAPI_GENERATOR_JAR").is_some() {
            return Err(format!("Generator JAR does not exist: {}", jar.display()).into());
        }
        fs::create_dir_all(jar.parent().unwrap())?;
        let download = Scratch(jar.with_extension(format!("download-{}", std::process::id())));
        let status = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--retry",
                "2",
            ])
            .arg(&source.generator_url)
            .arg("--output")
            .arg(&download.0)
            .status()?;
        if !status.success() {
            return Err("Could not download the pinned OpenAPI Generator JAR".into());
        }
        verify_hash(
            &fs::read(&download.0)?,
            &source.generator_sha256,
            "Generator JAR",
        )?;
        fs::rename(&download.0, &jar)?;
    }
    verify_hash(&fs::read(&jar)?, &source.generator_sha256, "Generator JAR")?;
    Ok(jar)
}

fn verify_hash(bytes: &[u8], expected: &str, label: &str) -> Result<()> {
    let actual = super::hex_digest(bytes);
    if actual != expected {
        return Err(format!("{label} checksum mismatch: expected {expected}, got {actual}").into());
    }
    Ok(())
}

fn apply_patches(schema: &mut Value, patches: Vec<Patch>) -> Result<()> {
    for patch in patches {
        let (parent, key) = patch.path.rsplit_once('/').ok_or("Invalid patch pointer")?;
        let target = schema
            .pointer_mut(parent)
            .ok_or_else(|| format!("Missing schema patch target: {}", patch.path))?;
        if key == "-" {
            let array = target
                .as_array_mut()
                .ok_or("Patch append target must be an array")?;
            if array.contains(&patch.value) {
                return Err(format!("Schema patch already present: {}", patch.path).into());
            }
            array.push(patch.value);
        } else {
            let object = target
                .as_object_mut()
                .ok_or("Patch target must be an object")?;
            let key = key.replace("~1", "/").replace("~0", "~");
            if object.get(&key) != patch.expected.as_ref() {
                return Err(format!("Schema patch precondition failed: {}", patch.path).into());
            }
            object.insert(key, patch.value);
        }
    }
    Ok(())
}

fn generated_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                visit(root, &entry?.path(), files)?;
            }
        } else {
            files.insert(path.strip_prefix(root)?.to_owned(), fs::read(path)?);
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, &root.join("src"), &mut files)?;
    visit(root, &root.join("Cargo.toml"), &mut files)?;
    Ok(files)
}

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = fs::remove_dir_all(&self.0);
        } else {
            let _ = fs::remove_file(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_rejects_missing_targets_and_reapplication() {
        let mut schema = serde_json::json!({"properties": {"field": {}}, "parameters": []});
        let patches = || {
            vec![
                Patch {
                    path: "/properties/field/x-nullable".into(),
                    expected: None,
                    value: true.into(),
                },
                Patch {
                    path: "/parameters/-".into(),
                    expected: None,
                    value: serde_json::json!({"name":"header"}),
                },
            ]
        };
        apply_patches(&mut schema, patches()).unwrap();
        assert_eq!(schema["properties"]["field"]["x-nullable"], true);
        assert_eq!(schema["parameters"].as_array().unwrap().len(), 1);
        assert!(apply_patches(&mut schema, patches()).is_err());
        assert!(apply_patches(&mut Value::Null, patches()).is_err());
    }

    #[test]
    fn altered_input_fails_checksum_validation() {
        let hash = super::super::hex_digest(b"pinned bytes");
        verify_hash(b"pinned bytes", &hash, "fixture").unwrap();
        assert!(verify_hash(b"changed bytes", &hash, "fixture").is_err());
    }
}
