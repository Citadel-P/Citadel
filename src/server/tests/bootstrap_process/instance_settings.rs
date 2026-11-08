use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_TEST_DATABASE_URL; starts real isolated Core processes"]
async fn saved_instance_choices_apply_after_restart_to_real_consumers() {
    let fixture = Fixture::new().await;
    let mut server = fixture.start_with_instance_environment(false, false, false, None);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], true);
    let initialized = setup::initialize(&fixture, &server, "instance-owner").await;
    assert_eq!(initialized.status(), 200);
    let token = initialized.json::<Value>().await.unwrap()["accessToken"]
        .as_str()
        .unwrap()
        .to_owned();
    let settings_url = format!("{}/api/v1/installation/settings", server.url);
    assert_eq!(
        fixture
            .client
            .get(format!("{}/swagger/", server.url))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let before: Value = fixture
        .client
        .get(&settings_url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(before["restartRequired"], false);
    let configuration = json!({
        "passwordMinimumLength": 40, "browserSessionLifetimeDays": 7,
        "serviceAccountDefaultTokenLifetimeDays": 10, "serviceAccountMaximumTokenLifetimeDays": 20,
        "serviceAccountMaximumActiveTokens": 3, "accessTokenLifetimeMinutes": 5,
        "statisticsCollectionIntervalSeconds": 17, "statisticsWriteIntervalSeconds": 19,
        "scriptsEnabled": false, "scriptsMaximumParallelRuns": 2, "scriptsMaximumRemoteRuns": 3,
        "scriptsDefaultTimeoutSeconds": 23, "scriptsMaximumTimeoutSeconds": 60,
        "buildPreparationSlots": 3, "buildTrackingSlots": 4, "buildCleanupEnabled": false, "buildRetentionDays": 13,
        "backupsEnabled": false, "backupTrackingSlots": 5, "backupDefaultTimeoutSeconds": 29,
        "openapiEnabled": true, "requireDirectAgentHttps": true,
        "agentImageOverride": "registry.example.test/citadel/agent:test", "logVerbosity": "debug", "logFormat": "Json"
    });
    let changed = fixture
        .client
        .patch(&settings_url)
        .bearer_auth(&token)
        .json(&json!({"configuration": configuration}))
        .send()
        .await
        .unwrap();
    assert_eq!(changed.status(), 200, "{}", changed.text().await.unwrap());
    let saved: Value = fixture
        .client
        .get(&settings_url)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(saved["configuration"], configuration);
    assert_eq!(
        saved["runningConfiguration"],
        before["runningConfiguration"]
    );
    assert_eq!(saved["restartRequired"], true);
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let expirations: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT id,expiresat FROM refreshtokens")
            .fetch_all(&pool)
            .await
            .unwrap();
    let first_logs = server.stop().await;
    assert!(
        !first_logs
            .lines()
            .any(|line| serde_json::from_str::<Value>(line).is_ok())
    );
    let mut restarted = fixture.start_with_instance_environment(false, false, false, None);
    assert_eq!(
        fixture.ready(&mut restarted).await["passwordMinimumLength"],
        40
    );
    assert_eq!(
        fixture
            .client
            .get(format!("{}/swagger/", restarted.url))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let active: Value = fixture
        .client
        .get(format!("{}/api/v1/installation/settings", restarted.url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(active["restartRequired"], false);
    assert_eq!(active["runningConfiguration"], configuration);
    let info: Value = fixture
        .client
        .get(format!("{}/api/v1/application/info", restarted.url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(info["scriptExecution"]["defaultTimeoutSeconds"], 23);
    assert_eq!(info["scriptExecution"]["maximumTimeoutSeconds"], 60);
    // Existing refresh credentials survive policy changes and retain their expirations.
    let after: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT id,expiresat FROM refreshtokens")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(expirations, after);
    let login = fixture
        .client
        .post(format!("{}/api/v1/authentication/login", restarted.url))
        .json(&json!({"emailOrName":"instance-owner","password":PASSWORD}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        login.status(),
        200,
        "an existing password continues to authenticate after policy changes"
    );
    let login: Value = login.json().await.unwrap();
    use base64::Engine;
    let access = login["accessToken"].as_str().unwrap();
    let claims: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(access.split('.').nth(1).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
        300
    );
    let lifetime: i64 = sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM expiresat-createdat)::bigint FROM refreshtokens ORDER BY createdat DESC LIMIT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(lifetime, 7 * 86400);
    let password_change = fixture
        .client
        .post(format!("{}/api/v1/profile/change-password", restarted.url))
        .bearer_auth(access)
        .json(&json!({"currentPassword":PASSWORD,"newPassword":"A-shorter-password22!"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        password_change.status(),
        400,
        "new passwords obey the active saved minimum"
    );
    let logs = restarted.stop().await;
    let effective = logs
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|line| line["fields"]["message"] == "Effective configuration")
        .expect("saved JSON/debug settings must reach the real logger");
    let effective: Value = serde_json::from_str(
        effective["fields"]["effective_configuration"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(effective["probeIntervalSeconds"], 17);
    assert_eq!(effective["openapiEnabled"], true);
    assert_eq!(
        effective["execution"]["agentImage"],
        configuration["agentImageOverride"]
    );
    assert_eq!(effective["statsFlushIntervalSeconds"], 19);
    assert_eq!(effective["passwordMinimumLength"], 40);
    assert_eq!(effective["refreshTokenLifetimeDays"], 7);
    assert_eq!(effective["accessTokenLifetimeMinutes"], 5);
    assert_eq!(
        effective["serviceAccountLimits"]["defaultTokenLifetimeDays"],
        10
    );
    assert_eq!(
        effective["serviceAccountLimits"]["maximumTokenLifetimeDays"],
        20
    );
    assert_eq!(
        effective["serviceAccountLimits"]["maximumActiveTokensPerAccount"],
        3
    );
    assert_eq!(effective["execution"]["agentAllowInsecure"], false);
    assert_eq!(
        effective["execution"]["scripts"],
        json!({"enabled":false,"maximumParallelRuns":2,"maximumRemoteRuns":3,"defaultTimeoutSeconds":23,"maximumTimeoutSeconds":60})
    );
    assert_eq!(effective["execution"]["buildPreparationSlots"], 3);
    assert_eq!(effective["execution"]["buildTrackingSlots"], 4);
    assert_eq!(effective["execution"]["buildRetentionDays"], Value::Null);
    assert_eq!(effective["execution"]["backups"]["enabled"], false);
    assert_eq!(effective["execution"]["backups"]["trackingSlots"], 5);
    assert_eq!(
        effective["execution"]["backups"]["defaultTimeoutSeconds"],
        29
    );
    pool.close().await;
    fixture.close().await;
}
