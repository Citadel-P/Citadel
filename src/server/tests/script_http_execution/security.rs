use super::*;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_identity::{ResourceAccessInput, UserRepository};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_scripts::{ScriptDraft, ScriptError};

pub(super) async fn verify(
    app: &Router,
    db: &PgPool,
    admin: &ActorPrincipal,
    store: &PostgresScriptRepository,
    root: &std::path::Path,
) {
    let caller = actor(db, false).await;
    let action = create(app, admin, "console.log('saved');", true, 10).await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/scripts/{id}/test");
    sqlx::query("UPDATE scripts SET runasactorid=$2,rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .bind(caller.actor_id.value())
        .execute(db)
        .await
        .unwrap();
    let users = PostgresUserRepository::new(db.clone());
    users
        .add_resource_access(
            caller.subject_id,
            &ResourceAccessInput {
                resource_type: ResourceType::Script,
                resource_id: id,
                permission_level: PermissionLevel::Execute,
                specific_permissions: vec![],
            },
            admin.actor_id,
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();

    let json_path = root.join("outside.json");
    let key_path = root.join("outside-key");
    let marker = "fixture-only-private-content-9384";
    tokio::fs::write(&json_path, json!({"value": marker}).to_string())
        .await
        .unwrap();
    tokio::fs::write(&key_path, marker).await.unwrap();
    let json_url = serde_json::to_string(&format!("file://{}", json_path.display())).unwrap();
    let key_url = serde_json::to_string(&format!("file://{}", key_path.display())).unwrap();
    let nested = serde_json::to_string(&format!("import {key_url};")).unwrap();
    for code in [
        format!("import data from {json_url} with {{type:'json'}}; console.log(data);"),
        format!("import {key_url};"),
        format!("await import({key_url});"),
        format!(
            "await Deno.writeTextFile('nested.ts', {nested}); const path = './nested.ts'; await import(path);"
        ),
        "await new Deno.Command('/bin/sh', {args:['-c','exit 0']}).output();".into(),
    ] {
        let response = request(
            app,
            Method::POST,
            &path,
            Some(caller.clone()),
            Some(json!({"code":code})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result = body(response).await;
        assert_eq!(
            result.as_array().unwrap().last().unwrap()["status"],
            "Failed",
            "{result}"
        );
        assert!(
            !result.to_string().contains(marker),
            "private fixture leaked"
        );
        assert!(
            result.to_string().contains("Requires"),
            "expected a permission denial: {result}"
        );
        let run = store.list_runs(id, 1).await.unwrap().remove(0);
        let run = store.get_run(id, run.id).await.unwrap();
        assert_eq!(run.run_as_actor_id, caller.actor_id.value());
        assert!(!run.logs.unwrap_or_default().contains(marker));
        assert!(!root.join(run.id.to_string()).exists());
    }
    // Ordinary modules created within this run still work through the bootstrap.
    let result = body(request(app, Method::POST, &path, Some(caller.clone()), Some(json!({
        "code":"await Deno.writeTextFile('local.ts', 'export const message = \"workspace-ok\";'); const local = './local.ts'; console.log((await import(local)).message);"
    }))).await).await;
    assert_eq!(
        result.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    assert!(result.to_string().contains("workspace-ok"));

    // Authorization is tied to the checked configuration, not a later row.
    let checked = store.get(id).await.unwrap();
    let draft = ScriptDraft {
        code: "console.log('stale');".into(),
        expected_row_version: checked.row_version,
    };
    let before = store.list_runs(id, 100).await.unwrap().len();
    sqlx::query("UPDATE scripts SET runasactorid=$2,rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .bind(admin.actor_id.value())
        .execute(db)
        .await
        .unwrap();
    assert!(matches!(
        store
            .enqueue_with_draft(caller.actor_id, id, "Test", &json!({}), None, Some(&draft))
            .await,
        Err(ScriptError::Conflict(_))
    ));
    assert_eq!(store.list_runs(id, 100).await.unwrap().len(), before);
    assert_eq!(
        store.get(id).await.unwrap().control_state,
        citadel_primitives::ResourceControlState::Idle
    );

    // Service Account draft execution requires Use; saved operation execution does not.
    let account = Uuid::now_v7();
    let account_actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'ServiceAccount')")
        .bind(account_actor)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO serviceaccounts(id,actorid,createdat,updatedat,createdbyactorid,name) VALUES($1,$2,NOW(),NOW(),$3,$4)")
        .bind(account).bind(account_actor).bind(admin.actor_id.value()).bind(format!("draft-{account}")).execute(db).await.unwrap();
    sqlx::query("UPDATE scripts SET runasactorid=$2,rowversion=rowversion+1 WHERE id=$1")
        .bind(id)
        .bind(account_actor)
        .execute(db)
        .await
        .unwrap();
    let input = json!({"code":"console.log('service-draft');"});
    assert_eq!(
        request(
            app,
            Method::POST,
            &path,
            Some(caller.clone()),
            Some(input.clone())
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let saved = body(request(app, Method::POST, &path, Some(caller.clone()), None).await).await;
    assert_eq!(
        saved.as_array().unwrap().last().unwrap()["status"],
        "Succeeded"
    );
    let access = ResourceAccessInput {
        resource_type: ResourceType::ServiceAccount,
        resource_id: account,
        permission_level: PermissionLevel::Read,
        specific_permissions: vec![SpecificPermission::Use],
    };
    users
        .add_resource_access(
            caller.subject_id,
            &access,
            admin.actor_id,
            chrono::Utc::now(),
            true,
        )
        .await
        .unwrap();
    let allowed = body(
        request(
            app,
            Method::POST,
            &path,
            Some(caller.clone()),
            Some(input.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(
        allowed.as_array().unwrap().last().unwrap()["status"],
        "Succeeded",
        "{allowed}"
    );
    users
        .remove_resource_access(
            caller.subject_id,
            &access,
            admin.actor_id,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(
        request(app, Method::POST, &path, Some(caller), Some(input))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    tokio::fs::remove_file(json_path).await.unwrap();
    tokio::fs::remove_file(key_path).await.unwrap();
}
