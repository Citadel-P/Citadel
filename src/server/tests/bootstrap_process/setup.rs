use super::*;

async fn initialize(fixture: &Fixture, server: &Server, name: &str) -> reqwest::Response {
    fixture
        .client
        .post(format!("{}/api/v1/setup/initialize", server.url))
        .json(&json!({"name":name,"email":format!("{name}@example.test"),"password":PASSWORD}))
        .send()
        .await
        .unwrap()
}

// SetupEndpointTests: pending status/API gate, former default credentials,
// Initialize_Should_Create_Admin_Complete_Setup_And_Start_Session, repeat rejection.
#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts an isolated Core process"]
async fn interactive_setup_persists_identity_session_and_exact_automation_defaults() {
    let fixture = Fixture::new().await;
    let mut server = fixture.start(false, false);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], true);
    let status = fixture
        .client
        .get(format!("{}/api/v1/setup/status", server.url))
        .send()
        .await
        .unwrap();
    assert_eq!(status.status(), 200);
    assert_eq!(status.headers()["cache-control"], "no-store");
    assert_eq!(status.json::<Value>().await.unwrap()["requiresSetup"], true);
    for response in [
        fixture
            .client
            .get(format!("{}/api/v1/application/info", server.url))
            .send()
            .await
            .unwrap(),
        fixture
            .client
            .post(format!("{}/api/v1/authentication/login", server.url))
            .json(&json!({"emailOrName":"admin@citadel.local","password":"admin123"}))
            .send()
            .await
            .unwrap(),
    ] {
        assert_eq!(response.status(), 409);
        assert_eq!(
            response.json::<Value>().await.unwrap()["type"],
            "setup_required"
        );
    }

    let response = initialize(&fixture, &server, "owner").await;
    assert_eq!(response.status(), 200);
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("refresh_token="))
        .expect("interactive setup must establish a refresh session")
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["nextStep"], "Completed");
    let token = body["accessToken"]
        .as_str()
        .filter(|token| !token.is_empty())
        .unwrap();
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    let users: Vec<(Uuid, Uuid, String, String, Uuid, bool, String)> = sqlx::query_as(
        "SELECT u.id,u.actorid,u.name,u.email,u.createdbyactorid,a.isenabled,a.type FROM users u JOIN actors a ON a.id=u.actorid"
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(users.len(), 1);
    let (id, actor, name, email, creator, enabled, kind) = &users[0];
    assert_eq!(name, "owner");
    assert_eq!(email, "owner@example.test");
    assert_eq!(*creator, Uuid::from_u128(1));
    assert!(*enabled);
    assert_eq!(kind, "User");
    let roles: Vec<Uuid> = sqlx::query_scalar("SELECT roleid FROM actorroles WHERE actorid=$1")
        .bind(actor)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        roles,
        vec![Uuid::parse_str("30000000-0000-0000-0000-000000000001").unwrap()]
    );
    let state: (Uuid, bool) = sqlx::query_as("SELECT initialadministratoractorid,initializedat IS NOT NULL FROM instancesetupstates WHERE id=1")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(state, (*actor, true));

    // Use the returned token through real authentication; no injected principal.
    let profile = fixture
        .client
        .get(format!("{}/api/v1/profile", server.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(profile.status(), 200);
    let profile: Value = profile.json().await.unwrap();
    assert_eq!(profile["id"], json!(id));
    assert_eq!(profile["displayName"], "owner");
    assert_eq!(profile["authorization"]["isAdministrator"], true);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], false);
    let sessions: Vec<Uuid> =
        sqlx::query_scalar("SELECT userid FROM refreshtokens WHERE expiresat > CURRENT_TIMESTAMP")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(sessions, vec![*id]);
    let refresh = fixture
        .client
        .get(format!("{}/api/v1/authentication/refresh", server.url))
        .header("cookie", cookie.split(';').next().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(refresh.status(), 200);
    assert!(
        !refresh.json::<Value>().await.unwrap()["accessToken"]
            .as_str()
            .unwrap()
            .is_empty()
    );

    let activities: Vec<(Uuid, String, Value)> = sqlx::query_as("SELECT createdbyactorid,resourcetype,info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='InitialAdministratorCreated'")
        .bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].0, Uuid::from_u128(1));
    assert_eq!(activities[0].1, "User");
    let info = &activities[0].2;
    assert_eq!(info["$type"], "InitialAdministratorCreated");
    assert_eq!(info["UserId"], json!(id));
    assert_eq!(info["UserName"], "owner");
    assert_eq!(info["Mode"], "Interactive");
    assert!(!info.to_string().contains(PASSWORD));

    let actions: Vec<Value> = sqlx::query_scalar(r#"
SELECT jsonb_build_object('name',a.name,'runAsActorId',a.runasactorid,
 'createdByActorId',a.createdbyactorid,'enabled',a.enabled,'scheduleEnabled',a.scheduleenabled,
 'scheduleCron',a.schedulecron,'tags',COALESCE((SELECT jsonb_agg(t.name ORDER BY t.name)
 FROM resourcetags rt JOIN tags t ON t.id=rt.tagid WHERE rt.resourceid=a.id AND rt.resourcetype='AutomationAction'),'[]'::jsonb))
FROM actions a ORDER BY a.name
"#).fetch_all(&pool).await.unwrap();
    assert_eq!(
        actions,
        vec![
            json!({"name":"Prune images","runAsActorId":actor,"createdByActorId":Uuid::from_u128(1),"enabled":false,"scheduleEnabled":false,"scheduleCron":"0 12 * * *","tags":["System"]}),
            json!({"name":"Restart unhealthy stacks","runAsActorId":actor,"createdByActorId":Uuid::from_u128(1),"enabled":false,"scheduleEnabled":false,"scheduleCron":"*/15 * * * *","tags":["Prod","System"]}),
        ]
    );

    let repeat = initialize(&fixture, &server, "other-owner").await;
    assert_eq!(repeat.status(), 409);
    assert_eq!(
        repeat.json::<Value>().await.unwrap()["type"],
        "setup_already_complete"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(!server.stop().await.contains(PASSWORD));
    pool.close().await;
    fixture.close().await;
}

// SetupEndpointTests.Deleting_Initial_User_Should_Not_Reopen_Setup.
#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission; starts isolated Core processes"]
async fn deleting_initial_user_does_not_reopen_setup_after_restart() {
    let fixture = Fixture::new().await;
    let mut server = fixture.start(false, false);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], true);
    assert_eq!(initialize(&fixture, &server, "owner").await.status(), 200);
    let pool = PgPool::connect(&fixture.database_url).await.unwrap();
    // Remove the fixture directly to exercise setup recovery. The User delete
    // endpoint must retain its last-administrator safety guard.
    let removed = sqlx::query("DELETE FROM users WHERE name='owner'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(removed.rows_affected(), 1);
    assert_eq!(fixture.ready(&mut server).await["requiresSetup"], false);
    server.stop().await;
    let mut restarted = fixture.start(false, false);
    assert_eq!(fixture.ready(&mut restarted).await["requiresSetup"], false);
    assert_eq!(
        initialize(&fixture, &restarted, "replacement")
            .await
            .status(),
        409
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    restarted.stop().await;
    pool.close().await;
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL; starts isolated Core processes"]
async fn configured_password_policy_applies_to_setup_users_changes_and_preserves_login() {
    let fixture = Fixture::new().await;
    let mut server = fixture.start_with_policy(false, false, true, 8);
    let status = fixture.ready(&mut server).await;
    assert_eq!(status["passwordMinimumLength"], 8);
    assert_eq!(status["passwordMaximumLength"], 128);
    for password in ["short", "12345678"] {
        let response = fixture
            .client
            .post(format!("{}/api/v1/setup/initialize", server.url))
            .json(&json!({"name":"owner","email":"owner@example.test","password":password}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
    }
    let response = fixture
        .client
        .post(format!("{}/api/v1/setup/initialize", server.url))
        .json(&json!({"name":"owner","email":"owner@example.test","password":"river oak"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let login: Value = response.json().await.unwrap();
    let token = login["accessToken"].as_str().unwrap();
    let response = fixture
        .client
        .post(format!("{}/api/v1/users", server.url))
        .bearer_auth(token)
        .json(&json!({"name":"operator","email":"operator@example.test","password":"maple sky"}))
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "{}",
        response.text().await.unwrap()
    );
    let response = fixture
        .client
        .post(format!("{}/api/v1/profile/change-password", server.url))
        .bearer_auth(token)
        .json(&json!({"currentPassword":"river oak","newPassword":"birch sky"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    server.stop().await;
    let mut server = fixture.start_with_policy(false, false, true, 15);
    assert_eq!(
        fixture.ready(&mut server).await["passwordMinimumLength"],
        15
    );
    // Raising the creation policy must not invalidate passwords already in use.
    let response = fixture
        .client
        .post(format!("{}/api/v1/authentication/login", server.url))
        .json(&json!({"emailOrName":"owner","password":"birch sky"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let login: Value = response.json().await.unwrap();
    let response = fixture
        .client
        .post(format!("{}/api/v1/profile/change-password", server.url))
        .bearer_auth(login["accessToken"].as_str().unwrap())
        .json(&json!({"currentPassword":"birch sky","newPassword":"pine leaf"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    server.stop().await;
    fixture.close().await;
}
