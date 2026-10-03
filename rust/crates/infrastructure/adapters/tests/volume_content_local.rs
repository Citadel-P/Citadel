use citadel_adapters::connectors::{
    docker::DockerClient, edge::EdgeRegistry, routing::volumes::content::VolumeContentAdapter,
};
use citadel_database::MigrationRunner;
use citadel_platforms::RuntimeErrorKind;
use citadel_runtime::DynamicTasks;
use futures_util::{FutureExt, StreamExt};
use sqlx::postgres::PgPoolOptions;
use std::{panic::AssertUnwindSafe, process::Command, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

fn docker(args: &[&str]) -> String {
    let output = Command::new("docker").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "Docker fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct Fixture(String);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = Command::new("docker").args(["rm", "-f", &self.0]).output();
        let _ = Command::new("docker")
            .args(["volume", "rm", &self.0])
            .output();
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL, Docker and an image containing citadel-volume-helper"]
async fn local_volume_browser_uses_core_image_without_pulling_agent() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let fixture = Fixture(format!("citadel-volume-test-{platform}"));
    let image =
        std::env::var("CITADEL_VOLUME_TEST_IMAGE").unwrap_or_else(|_| "citadel-wsl-core".into());
    docker(&["volume", "create", &fixture.0]);
    docker(&[
        "run",
        "--rm",
        "--user",
        "0",
        "--entrypoint",
        "sh",
        "-v",
        &format!("{}:/data", fixture.0),
        &image,
        "-c",
        "mkdir /data/folder; printf 'binary\\000\\377' > /data/folder/file; ln -s /etc /data/escape",
    ]);
    // An inspected Core container supplies its immutable image, even when its
    // tag changes or the remote Agent registry is unavailable.
    docker(&[
        "create",
        "--name",
        &fixture.0,
        "--label",
        "com.citadel.system-role=core",
        "--entrypoint",
        "/usr/local/bin/citadel-volume-helper",
        &image,
        "idle",
    ]);
    sqlx::query("INSERT INTO platforms (id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,'http://localhost.docker','Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(&fixture.0).execute(&pool).await.unwrap();
    let cancel = CancellationToken::new();
    let tasks = DynamicTasks::new(cancel.clone());
    let client = DockerClient::new(
        std::env::var("CITADEL_RUST_DOCKER_SOCKET")
            .unwrap_or_else(|_| "/var/run/docker.sock".into()),
        Duration::from_secs(10),
    )
    .unwrap();
    let adapter = || {
        VolumeContentAdapter::new(
            pool.clone(),
            client.clone(),
            None,
            EdgeRegistry::default(),
            "citadel-missing-volume-helper:does-not-exist".into(),
            tasks.clone(),
        )
    };
    let result = AssertUnwindSafe(async {
        let browser = adapter().with_core_container(fixture.0.clone());
        let listing = browser
            .list(platform, &fixture.0, "/", None, &cancel)
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), 2);
        assert_eq!(listing.entries[0].name, "folder");
        let mut download = browser
            .download(platform, &fixture.0, "/folder/file", None, &cancel)
            .await
            .unwrap();
        let mut bytes = Vec::new();
        while let Some(chunk) = download.stream.next().await {
            bytes.extend(chunk.unwrap());
        }
        assert_eq!(bytes, b"binary\0\xff");
        assert_eq!(
            browser
                .list(platform, &fixture.0, "/missing", None, &cancel)
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::NotFound
        );
        assert_eq!(
            browser
                .list(platform, &fixture.0, "/escape", None, &cancel)
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::InvalidRequest
        );
        assert_eq!(
            adapter()
                .list(platform, &fixture.0, "/", None, &cancel)
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::Unavailable
        );
        assert_eq!(
            browser
                .list(
                    platform,
                    &format!("{}-absent", fixture.0),
                    "/",
                    None,
                    &cancel
                )
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::NotFound
        );
    })
    .catch_unwind()
    .await;
    tasks.drain(Duration::from_secs(15)).await.unwrap();
    assert!(
        docker(&[
            "ps",
            "-aq",
            "--filter",
            &format!("label=com.citadel.platform-id={platform}"),
            "--filter",
            "label=com.citadel.system-role=volume-helper"
        ])
        .is_empty()
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
