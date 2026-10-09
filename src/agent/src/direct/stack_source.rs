//! Prepare bounded source files and versioned mounts without database ownership.
use citadel_contracts::citadel::stacks::v1::StackApplyRequest;
use citadel_stacks::StackApplySource;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};
use tonic::Status;

const MAX_SOURCE_BYTES: usize = 12 * 1024 * 1024;

pub(super) struct Prepared {
    pub root: PathBuf,
    pub source: StackApplySource,
    pub environment: Vec<String>,
    pub secret_directory: Option<PathBuf>,
    persistent_root: PathBuf,
    keep_secrets: bool,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
        if !self.keep_secrets
            && let Some(path) = &self.secret_directory
        {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
impl Prepared {
    pub fn retain_secrets(&mut self) {
        self.keep_secrets = true;
        // Only remove a directory previously recorded by this Agent, never
        // unrelated files in a caller-provided generated-files directory.
        let marker = self.persistent_root.join(".citadel-mounted-secrets");
        let previous = read_text(&marker, 128).ok();
        let current = self
            .secret_directory
            .as_ref()
            .and_then(|v| v.file_name())
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        if private_directory(&self.persistent_root).is_ok()
            && write(&marker, current.as_bytes()).is_ok()
            && let Some(previous) = previous.filter(|v| !v.is_empty() && v != current)
            && relative(&previous).is_ok()
            && Path::new(&previous).components().count() == 1
        {
            let _ = std::fs::remove_dir_all(self.persistent_root.join("secrets").join(previous));
        }
    }
}
fn io(error: std::io::Error) -> Status {
    Status::internal(format!("Stack source I/O failed: {error}"))
}
fn read_text(path: &Path, limit: usize) -> Result<String, Status> {
    use std::io::Read;
    let mut content = Vec::new();
    std::fs::File::open(path)
        .map_err(io)?
        .take(limit as u64 + 1)
        .read_to_end(&mut content)
        .map_err(io)?;
    if content.len() > limit {
        return Err(Status::resource_exhausted(
            "Stack source file exceeds its byte budget",
        ));
    }
    String::from_utf8(content)
        .map_err(|error| io(std::io::Error::new(std::io::ErrorKind::InvalidData, error)))
}
fn relative(value: &str) -> Result<PathBuf, Status> {
    if value.trim().is_empty()
        || value.contains('\\')
        || value.contains(':')
        || Path::new(value)
            .components()
            .any(|p| !matches!(p, Component::Normal(_) | Component::CurDir))
    {
        return Err(Status::invalid_argument(
            "Source paths must be relative and cannot escape their root",
        ));
    }
    Ok(PathBuf::from(value))
}
fn private_directory(path: &Path) -> Result<(), Status> {
    std::fs::create_dir_all(path).map_err(io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    Ok(())
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), Status> {
    if let Some(parent) = path.parent() {
        private_directory(parent)?;
    }
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(io)
}
fn add_override(prepared: &mut Prepared, name: &str, value: &Value) -> Result<(), Status> {
    let path = prepared.root.join(format!(".citadel/{name}.json"));
    write(&path, &serde_json::to_vec(value).expect("JSON value"))?;
    prepared
        .source
        .compose_paths
        .push(path.display().to_string());
    Ok(())
}
pub(super) fn prepare(r: &StackApplyRequest, base: &Path) -> Result<Prepared, Status> {
    if r.stack_name.trim().is_empty() {
        return Err(Status::invalid_argument("Stack name is required"));
    }
    let swarm = r.orchestration_mode == 1;
    if !matches!(r.orchestration_mode, 0 | 1) {
        return Err(Status::invalid_argument("Invalid orchestration mode"));
    }
    if !swarm
        && (r.convert_compose_project_to_swarm
            || !r.retained_swarm_secrets.is_empty()
            || !r.retained_swarm_configs.is_empty())
    {
        return Err(Status::invalid_argument(
            "Retained Swarm resources and conversion require Swarm orchestration",
        ));
    }
    if swarm
        && (r.destroy_before_deploy
            || r.pre_deploy.is_some()
            || r.post_deploy.is_some()
            || !r.service_names.is_empty())
    {
        return Err(Status::invalid_argument(
            "Swarm apply does not support service selection or pre/post commands",
        ));
    }
    if r.source_files.len() > 512
        || r.source_env_file_paths.len() > 512
        || r.source_files
            .iter()
            .map(|v| v.content.len())
            .sum::<usize>()
            > MAX_SOURCE_BYTES
    {
        return Err(Status::resource_exhausted(
            "Stack source exceeds 512 files or 12 MiB",
        ));
    }
    let mut paths = HashSet::new();
    for f in &r.source_files {
        let path = relative(&f.relative_path)?;
        if !paths.insert(path) {
            return Err(Status::invalid_argument("Duplicate source file path"));
        }
    }
    // Hash the identity to avoid path traversal and sanitized-name collisions.
    let persistent_root = base.join(hex_name(&r.stack_name));
    let root = base.join("runs").join(uuid::Uuid::now_v7().to_string());
    private_directory(&root)?;
    let mut p = Prepared {
        root,
        persistent_root,
        secret_directory: None,
        keep_secrets: false,
        environment: Vec::new(),
        source: StackApplySource {
            files: vec![],
            compose_paths: vec![],
            env_file_paths: vec![],
            working_directory: ".".into(),
            labels_override_path: None,
            resolved_commit_sha: None,
        },
    };
    let transported = !r.source_files.is_empty();
    for f in &r.source_files {
        write(&p.root.join(relative(&f.relative_path)?), &f.content)?;
    }
    let source_backed = r
        .source_working_directory
        .as_ref()
        .is_some_and(|v| !v.is_empty())
        && !r.source_compose_file_paths.is_empty();
    let resolve = |value: &str| -> Result<String, Status> {
        if transported {
            return Ok(p.root.join(relative(value)?).display().to_string());
        }
        let path = Path::new(value);
        Ok(if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().map_err(io)?.join(path)
        }
        .display()
        .to_string())
    };
    if source_backed {
        p.source.working_directory = resolve(r.source_working_directory.as_deref().unwrap_or("."))?;
        p.source.compose_paths = r
            .source_compose_file_paths
            .iter()
            .map(|v| resolve(v))
            .collect::<Result<_, _>>()?;
        p.source.env_file_paths = r
            .source_env_file_paths
            .iter()
            .map(|v| resolve(v))
            .collect::<Result<_, _>>()?;
        p.source.labels_override_path = r
            .labels_override_file_path
            .as_deref()
            .filter(|v| !v.is_empty())
            .map(resolve)
            .transpose()?;
    } else {
        let content = r
            .compose_file_content
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| Status::invalid_argument("Compose content is required"))?;
        write(&p.root.join("compose.yml"), content.as_bytes())?;
        p.source.compose_paths.push("compose.yml".into());
    }
    if let Some(path) = r
        .generated_files_directory
        .as_deref()
        .filter(|v| !v.is_empty())
    {
        // External callers can choose their persistent mount location. Transported
        // paths remain confined to the staging tree and use the stable mount root.
        if transported {
            relative(path)?;
        } else {
            p.persistent_root = PathBuf::from(resolve(path)?);
        }
    }
    let mut remaining = MAX_SOURCE_BYTES;
    for path in &p.source.env_file_paths {
        let content = read_text(Path::new(path), remaining)?;
        remaining -= content.len();
        for line in content.lines().filter(|v| !v.trim_start().starts_with('#')) {
            if line.split_once('=').is_some() {
                remaining = remaining
                    .checked_sub(std::mem::size_of::<String>())
                    .ok_or_else(|| {
                        Status::resource_exhausted("Stack environment exceeds its byte budget")
                    })?;
                p.environment.push(line.trim().to_owned());
            }
        }
    }
    for value in &r.environment_variables {
        remaining = remaining
            .checked_sub(value.len().saturating_add(std::mem::size_of::<String>()))
            .ok_or_else(|| {
                Status::resource_exhausted("Stack environment exceeds its byte budget")
            })?;
        p.environment.push(value.clone());
    }
    if let Some(path) = r.environment_file_path.as_deref().filter(|v| !v.is_empty()) {
        let path = p.root.join(relative(path)?);
        write(&path, p.environment.join("\n").as_bytes())?;
        p.source.env_file_paths.push(path.display().to_string());
    }
    if !r.retained_swarm_secrets.is_empty() {
        let mut doc = json!({"services":{},"secrets":{}});
        for v in &r.retained_swarm_secrets {
            doc["secrets"][&v.compose_resource_name] =
                json!({"external":true,"name":v.docker_resource_name});
            for m in &v.mounts {
                append_mount(
                    &mut doc,
                    &m.service_name,
                    "secrets",
                    json!({"source":v.compose_resource_name,"target":m.target_name}),
                );
            }
        }
        add_override(&mut p, "retained-secrets", &doc)?;
    } else if !r.secret_files.is_empty() {
        if r.secret_target_service_names.is_empty() {
            return Err(Status::invalid_argument(
                "Mounted secrets require target services",
            ));
        }
        let mut hash = Sha256::new();
        for v in &r.secret_files {
            for value in [&v.name, &v.target_path, &v.content] {
                hash.update((value.len() as u64).to_le_bytes());
                hash.update(value);
            }
        }
        let directory = if swarm {
            p.root.join("secrets")
        } else {
            p.persistent_root.join("secrets").join(format!(
                "{}-{}",
                hex(&hash.finalize()),
                uuid::Uuid::now_v7()
            ))
        };
        // A unique directory prevents a failed concurrent apply from removing
        // another deployment's mounted files.
        private_directory(&directory)?;
        if !swarm {
            p.secret_directory = Some(directory.clone());
        }
        let mut doc = json!({"services":{}});
        let mut names = HashSet::new();
        for v in &r.secret_files {
            let name = relative(&v.name)?;
            if name.components().count() != 1
                || !names.insert(name.clone())
                || !v.target_path.starts_with('/')
            {
                return Err(Status::invalid_argument(
                    "Invalid or duplicate secret name/target",
                ));
            }
            let path = directory.join(name);
            write(&path, v.content.as_bytes())?;
            let key = format!(
                "citadel-{}-{}",
                &hex_name(&v.name)[..16],
                &hex_name(&v.content)[..16]
            );
            if swarm {
                doc["secrets"][&key] = json!({"file":path.display().to_string()});
            }
            for service in &r.secret_target_service_names {
                let value = if swarm {
                    json!({"source":key,"target":Path::new(&v.target_path).file_name().and_then(|v|v.to_str()).unwrap_or(&v.name)})
                } else {
                    json!({"type":"bind","source":path.display().to_string(),"target":v.target_path,"read_only":true})
                };
                append_mount(
                    &mut doc,
                    service,
                    if swarm { "secrets" } else { "volumes" },
                    value,
                );
            }
        }
        add_override(&mut p, "mounted-secrets", &doc)?;
    }
    if !r.retained_swarm_configs.is_empty() {
        let mut doc = json!({"services":{},"configs":{}});
        for v in &r.retained_swarm_configs {
            doc["configs"][&v.compose_resource_name] =
                json!({"external":true,"name":v.docker_resource_name});
            for m in &v.mounts {
                append_mount(
                    &mut doc,
                    &m.service_name,
                    "configs",
                    json!({"source":v.compose_resource_name,"target":m.target_name}),
                );
            }
        }
        add_override(&mut p, "retained-configs", &doc)?;
    }
    Ok(p)
}
fn append_mount(doc: &mut Value, service: &str, kind: &str, value: Value) {
    if !doc["services"][service][kind].is_array() {
        doc["services"][service][kind] = json!([]);
    }
    doc["services"][service][kind]
        .as_array_mut()
        .expect("array")
        .push(value);
}
fn hex_name(name: &str) -> String {
    hex(&Sha256::digest(name.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_contracts::citadel::stacks::v1::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "citadel-stack-source-test-{}",
                uuid::Uuid::now_v7()
            )))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn request() -> StackApplyRequest {
        StackApplyRequest {
            stack_name: "cache".into(),
            compose_file_content: Some("services: {cache: {image: redis}}".into()),
            ..Default::default()
        }
    }
    #[test]
    fn local_environment_files_share_a_bounded_read_budget() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(&fixture.0).unwrap();
        let path = fixture.0.join("large.env");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len((MAX_SOURCE_BYTES / 2 + 1) as u64).unwrap();
        let mut r = request();
        r.source_working_directory = Some(fixture.0.display().to_string());
        r.source_compose_file_paths = vec!["compose.yml".into()];
        r.source_env_file_paths = vec![path.display().to_string(); 2];
        assert_eq!(
            prepare(&r, &fixture.0).err().unwrap().code(),
            tonic::Code::ResourceExhausted
        );
        file.set_len(MAX_SOURCE_BYTES as u64 + 1).unwrap();
        r.source_env_file_paths.truncate(1);
        assert_eq!(
            prepare(&r, &fixture.0).err().unwrap().code(),
            tonic::Code::ResourceExhausted
        );
    }
    #[test]
    fn bounded_text_checks_size_before_decoding_a_split_utf8_character() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(&fixture.0).unwrap();
        let path = fixture.0.join("unicode.env");
        std::fs::write(&path, "A=é").unwrap();
        assert_eq!(read_text(&path, 4).unwrap(), "A=é");
        assert_eq!(
            read_text(&path, 2).unwrap_err().code(),
            tonic::Code::ResourceExhausted
        );
    }
    #[test]
    fn transported_paths_counts_duplicates_and_escaping_are_rejected_before_writes() {
        let fixture = Fixture::new();
        for path in [
            "../escape",
            "/absolute",
            "nested/../../escape",
            "C:\\escape",
            "a\\..\\escape",
        ] {
            let mut r = request();
            r.source_files = vec![StackSourceFile {
                relative_path: path.into(),
                content: vec![],
            }];
            assert_eq!(
                prepare(&r, &fixture.0).err().unwrap().code(),
                tonic::Code::InvalidArgument
            );
            assert!(!fixture.0.exists());
        }
        let mut r = request();
        r.source_files = vec![
            StackSourceFile {
                relative_path: "compose.yml".into(),
                content: vec![]
            };
            513
        ];
        assert_eq!(
            prepare(&r, &fixture.0).err().unwrap().code(),
            tonic::Code::ResourceExhausted
        );
        r.source_files.truncate(2);
        assert_eq!(
            prepare(&r, &fixture.0).err().unwrap().code(),
            tonic::Code::InvalidArgument
        );
    }
    #[test]
    fn staged_sources_preserve_bytes_and_cleanup_on_drop() {
        let fixture = Fixture::new();
        let mut r = request();
        r.source_working_directory = Some("project".into());
        r.source_compose_file_paths = vec!["project/compose.yml".into()];
        r.source_files = vec![
            StackSourceFile {
                relative_path: "project/compose.yml".into(),
                content: b"services: {}".to_vec(),
            },
            StackSourceFile {
                relative_path: "project/binary".into(),
                content: vec![0, 255, 1],
            },
        ];
        let prepared = prepare(&r, &fixture.0).unwrap();
        let root = prepared.root.clone();
        assert_eq!(
            std::fs::read(root.join("project/binary")).unwrap(),
            [0, 255, 1]
        );
        assert!(Path::new(&prepared.source.working_directory).is_dir());
        drop(prepared);
        assert!(!root.exists());
    }
    #[test]
    fn successful_compose_keeps_private_mounts_and_only_prunes_its_previous_version() {
        let fixture = Fixture::new();
        let mut r = request();
        r.secret_files = vec![StackSecretFile {
            name: "token".into(),
            target_path: "/run/secrets/token".into(),
            content: "private-value".into(),
        }];
        r.secret_target_service_names = vec!["cache".into()];
        let mut first = prepare(&r, &fixture.0).unwrap();
        let path = first.secret_directory.clone().unwrap();
        let unrelated = first.persistent_root.join("secrets/unrelated");
        std::fs::create_dir_all(&unrelated).unwrap();
        assert_eq!(
            std::fs::read_to_string(path.join("token")).unwrap(),
            "private-value"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path.join("token"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        first.retain_secrets();
        drop(first);
        assert!(path.exists());
        r.secret_files[0].content = "new-private-value".into();
        let mut second = prepare(&r, &fixture.0).unwrap();
        let next = second.secret_directory.clone().unwrap();
        second.retain_secrets();
        drop(second);
        assert!(next.exists());
        assert!(!path.exists());
        assert!(unrelated.exists());
        let failed = prepare(&r, &fixture.0).unwrap();
        let path = failed.secret_directory.clone().unwrap();
        drop(failed);
        assert!(!path.exists());
        assert!(next.exists());
    }
    #[test]
    fn retained_swarm_materials_are_external_and_preserve_per_service_targets() {
        let fixture = Fixture::new();
        let mut r = request();
        r.orchestration_mode = 1;
        r.retained_swarm_secrets = vec![StackRetainedSwarmSecret {
            compose_resource_name: "credentials".into(),
            docker_resource_name: "live-secret".into(),
            mounts: vec![StackRetainedSwarmSecretMount {
                service_name: "cache".into(),
                target_name: "password".into(),
            }],
        }];
        r.retained_swarm_configs = vec![StackRetainedSwarmConfig {
            compose_resource_name: "settings".into(),
            docker_resource_name: "live-config".into(),
            mounts: vec![StackRetainedSwarmConfigMount {
                service_name: "cache".into(),
                target_name: "/etc/cache.conf".into(),
            }],
        }];
        let p = prepare(&r, &fixture.0).unwrap();
        let secrets: Value = serde_json::from_slice(
            &std::fs::read(p.root.join(".citadel/retained-secrets.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            secrets["secrets"]["credentials"],
            json!({"external":true,"name":"live-secret"})
        );
        assert_eq!(
            secrets["services"]["cache"]["secrets"][0]["target"],
            "password"
        );
        let configs: Value = serde_json::from_slice(
            &std::fs::read(p.root.join(".citadel/retained-configs.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            configs["services"]["cache"]["configs"][0]["target"],
            "/etc/cache.conf"
        );
    }
}
