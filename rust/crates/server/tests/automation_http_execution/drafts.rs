use super::*;

pub(super) async fn verify(
    app: &Router,
    db: &PgPool,
    admin: &ActorPrincipal,
    store: &PostgresAutomationRepository,
    action: &Value,
) {
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/automation/actions/{id}/test");
    let code = "console.log('draft-only', args.message);";
    let writer = actor(db, false).await;
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,2,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(writer.actor_id.value()).bind(id)
        .bind(citadel_primitives::ResourceType::AutomationAction as i32)
        .execute(db).await.unwrap();
    assert_eq!(
        request(
            app,
            Method::POST,
            &path,
            Some(writer.clone()),
            Some(json!({"code":code}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN,
        "Write permission alone must not allow running a draft"
    );
    assert!(store.list_runs(id, 10).await.unwrap().is_empty());
    for invalid in [" ".to_owned(), "x".repeat(256 * 1024 + 1)] {
        let result = body(
            request(
                app,
                Method::POST,
                &path,
                Some(admin.clone()),
                Some(json!({"code":invalid})),
            )
            .await,
        )
        .await;
        assert_eq!(result[0]["error"]["code"], 400);
        assert!(store.list_runs(id, 10).await.unwrap().is_empty());
    }
    let tested = body(
        request(
            app,
            Method::POST,
            &path,
            Some(admin.clone()),
            Some(json!({
                "code":code,"argsJson":"{\"message\":\"edited\"}","timeoutSeconds":1
            })),
        )
        .await,
    )
    .await;
    assert_eq!(
        tested.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    assert!(tested.to_string().contains("draft-only edited"));
    assert!(!tested.to_string().contains("disabled test"));
    let latest = store.list_runs(id, 1).await.unwrap().remove(0);
    let run = store.get_run(id, latest.id).await.unwrap();
    assert_eq!(run.code_snapshot.as_deref(), Some(code));
    assert_eq!(run.code_hash, citadel_automation::code_hash(code));
    assert_eq!(
        serde_json::from_str::<Value>(&run.args_json).unwrap(),
        json!({"message":"edited"})
    );
    assert_eq!(run.timeout_seconds, 10);
    assert_eq!(run.trigger, "Test");
    let saved = store.get(id).await.unwrap();
    assert_eq!(saved.code, action["code"].as_str().unwrap());
    assert_eq!(
        saved.default_args_json,
        action["defaultArgsJson"].as_str().unwrap()
    );
    assert_eq!(saved.run_as_actor_id, run.run_as_actor_id);
    assert!(!saved.enabled);

    // A resource-specific Execute grant allows tests; omission still uses saved code.
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=4 WHERE actorid=$1 AND resourceid=$2")
        .bind(writer.actor_id.value())
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    let tested = body(request(app, Method::POST, &path, Some(writer), None).await).await;
    assert_eq!(
        tested.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    assert!(tested.to_string().contains("disabled test"));
}
