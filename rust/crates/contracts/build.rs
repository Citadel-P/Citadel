#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct ProtoSource {
    directory: String,
    files: Vec<ProtoFile>,
}

#[derive(Deserialize)]
struct ProtoFile {
    name: String,
    sha256: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_directory = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let source_file = crate_directory.join("proto/source.json");
    println!("cargo:rerun-if-changed={}", source_file.display());

    let source: ProtoSource = serde_json::from_slice(&fs::read(&source_file)?)?;
    let proto_directory = normalize(crate_directory.join(source.directory));
    let mut protos = Vec::with_capacity(source.files.len());
    let mut edge_protos = Vec::new();
    for file in source.files {
        let path = proto_directory.join(&file.name);
        println!("cargo:rerun-if-changed={}", path.display());
        verify_checksum(&path, &file.sha256)?;
        if file.name == "edge_agent_service.proto" {
            edge_protos.push(path);
        } else {
            protos.push(path);
        }
    }

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let protobuf_include = protoc_bin_vendored::include_path()?;
    let mut prost = prost_build::Config::new();
    prost.protoc_executable(&protoc);

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_with_config(
            prost,
            &protos,
            &[proto_directory.clone(), protobuf_include.clone()],
        )?;

    // The Edge RPC itself is named Connect. Do not generate the identically
    // named convenience Channel constructor; callers supply a Channel to new().
    let mut prost = prost_build::Config::new();
    prost.protoc_executable(protoc);
    tonic_prost_build::configure()
        .build_transport(false)
        .compile_with_config(prost, &edge_protos, &[proto_directory, protobuf_include])?;

    Ok(())
}

fn verify_checksum(path: &Path, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let actual = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual != expected {
        return Err(format!(
            "protobuf checksum mismatch for {}: expected {expected}, got {actual}",
            path.display()
        )
        .into());
    }
    Ok(())
}

fn normalize(path: PathBuf) -> PathBuf {
    path.components().collect()
}
