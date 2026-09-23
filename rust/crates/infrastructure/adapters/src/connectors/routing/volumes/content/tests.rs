use super::*;
use citadel_contracts::citadel::containers::v1::{ExecExit, ExecOutput};

fn output(data: Vec<u8>) -> ExecServerMessage {
    ExecServerMessage {
        msg: Some(Msg::Output(ExecOutput { data, stream: 0 })),
    }
}
fn exit(code: i32) -> ExecServerMessage {
    ExecServerMessage {
        msg: Some(Msg::Exit(ExecExit { exit_code: code })),
    }
}
fn frames(values: Vec<ExecServerMessage>) -> Frames {
    Box::pin(futures_util::stream::iter(values.into_iter().map(Ok)))
}

#[tokio::test]
async fn helper_metadata_requires_successful_exit_and_bounds_output() {
    let cancel = CancellationToken::new();
    let json=br#"{"Path":"/","Entries":[{"Name":"folder","Path":"/folder","Type":"directory"}],"IsTruncated":false}"#.to_vec();
    let listing: Listing = metadata(frames(vec![output(json.clone()), exit(0)]), &cancel)
        .await
        .unwrap();
    assert_eq!(listing.entries[0].entry_type, VolumeEntryType::Directory);
    assert_eq!(
        serde_json::to_value(&listing.entries[0]).unwrap()["type"],
        "Directory"
    );
    for values in [
        vec![output(json.clone())],
        vec![output(json.clone()), exit(1)],
        vec![output(json.clone()), exit(0), exit(0)],
        vec![exit(0), output(json)],
        vec![output(vec![b'x'; DIRECTORY_PAYLOAD_LIMIT + 1]), exit(0)],
    ] {
        assert!(metadata::<Listing>(frames(values), &cancel).await.is_err());
    }
}

#[test]
fn volume_helper_request_is_read_only_and_cannot_execute_user_shell_input() {
    let request = helper_request(
        Uuid::now_v7(),
        "data",
        "citadel-volume-helper-test",
        "agent@sha256:digest",
        HELPER_BINARY,
    );
    assert_eq!(request.command, ["volume-helper", "idle"]);
    assert_eq!(request.entry_point, [HELPER_BINARY]);
    assert!(request.env_vars.is_empty());
    assert!(request.ports.is_empty());
    assert!(request.networks.is_empty());
    assert_eq!(request.mounts.len(), 1);
    assert_eq!(request.mounts[0].read_only, Some(true));
    let local = local_request(&request);
    assert_eq!(local["HostConfig"]["ReadonlyRootfs"], true);
    assert_eq!(local["HostConfig"]["Mounts"][0]["ReadOnly"], true);
    assert_eq!(local["HostConfig"]["NetworkMode"], "none");
    assert_eq!(local["Healthcheck"]["Test"], serde_json::json!(["NONE"]));
    assert_eq!(local["Labels"]["com.citadel.system"], "true");
}

#[test]
fn local_helper_uses_the_running_core_image_and_bundled_binary() {
    let image = format!("sha256:{}", "a".repeat(64));
    let document = serde_json::json!({
        "Image": image, "Config": {"Image": "mutable:tag", "Labels": {"com.citadel.system-role": "core"}}
    });
    assert_eq!(core_image(&document).unwrap(), image);
    let request = helper_request(
        Uuid::now_v7(),
        "data",
        "helper",
        &core_image(&document).unwrap(),
        CORE_HELPER_BINARY,
    );
    assert_eq!(request.image_id, image);
    assert_eq!(request.entry_point, [CORE_HELPER_BINARY]);
    for invalid in [
        serde_json::json!({"Image": image}),
        serde_json::json!({"Config":{"Labels":{"com.citadel.system-role":"core"}}, "Image":"mutable:tag"}),
    ] {
        assert_eq!(
            core_image(&invalid).unwrap_err().kind,
            RuntimeErrorKind::Unavailable
        );
    }
}

#[test]
fn missing_helper_image_is_unavailable_not_a_missing_volume_path() {
    assert_eq!(
        helper_startup_error("No such image".into()).kind,
        RuntimeErrorKind::Unavailable
    );
    assert_eq!(
        check_helper_error(Some("VolumePathNotFound"))
            .unwrap_err()
            .kind,
        RuntimeErrorKind::NotFound
    );
}
