#![forbid(unsafe_code)]

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_directory = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_directory = crate_directory.join("proto");
    let output_directory = PathBuf::from(std::env::var("OUT_DIR")?);
    let protos: Vec<_> = [
        "shared_models.proto",
        "platform_service.proto",
        "container_service.proto",
        "deployment_service.proto",
        "stack_service.proto",
        "image_service.proto",
        "network_service.proto",
        "volume_service.proto",
        "swarm_service.proto",
    ]
    .into_iter()
    .map(|name| proto_directory.join(name))
    .collect();
    let edge_protos = [proto_directory.join("edge_agent_service.proto")];
    for path in protos.iter().chain(&edge_protos) {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let protobuf_include = protoc_bin_vendored::include_path()?;
    let mut prost = prost_build::Config::new();
    prost.protoc_executable(&protoc);

    let mut builder = tonic_prost_build::configure();
    // Only inspection messages need a JSON document for adoption comparison.
    for name in [
        "InspectContainerResponse",
        "HostPortBindingList",
        "HostPortBinding",
        "NetworkSettings",
        "MapFieldNetwork",
        "EndpointSettings",
        "EndpointIPAMConfig",
        "Address",
        "ContainerConfig",
        "MountPoint",
        "GraphDriverData",
        "HostConfig",
        "Ulimits",
        "Mount",
        "VolumeOptions",
        "DriverConfig",
        "BindOptions",
        "RestartPolicy",
        "ContainerState",
        "LogConfig",
        "Health",
    ] {
        builder = builder.type_attribute(
            format!(".citadel.shared_models.v1.{name}"),
            "#[derive(serde::Serialize)]",
        );
    }
    builder
        .file_descriptor_set_path(output_directory.join("direct.bin"))
        .build_client(true)
        .build_server(true)
        .type_attribute(
            ".citadel.images.v1.InspectImageResponse",
            "#[derive(serde::Serialize)]",
        )
        .type_attribute(
            ".citadel.images.v1.ContainerImageResult",
            "#[derive(serde::Serialize)]",
        )
        .type_attribute(
            ".citadel.images.v1.HistoryImageResult",
            "#[derive(serde::Serialize)]",
        )
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
        .file_descriptor_set_path(output_directory.join("edge.bin"))
        .build_transport(false)
        .compile_with_config(prost, &edge_protos, &[proto_directory, protobuf_include])?;

    Ok(())
}
