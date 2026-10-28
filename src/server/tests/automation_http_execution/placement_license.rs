use super::*;
use std::sync::atomic::Ordering;

pub(super) async fn verify(
    app: &Router,
    admin: &ActorPrincipal,
    store: &PostgresAutomationRepository,
    service: &AutomationService,
    license: &webhooks::Entitlement,
) {
    let payload = json!({
        "name": format!("automatic-{}", Uuid::now_v7()),
        "code": "console.log('automatic');", "enabled": true,
        "executor": {"kind": "Automatic"}, "scheduleEnabled": false,
        "alertOnFailure": false, "timeoutSeconds": 10
    });
    let create_path = "/api/v1/automation/actions";
    let denied = request(
        app,
        Method::POST,
        create_path,
        Some(admin.clone()),
        Some(payload.clone()),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(body(denied).await["capability"], "automated-operations");

    license.0.store(true, Ordering::Relaxed);
    let created = request(
        app,
        Method::POST,
        create_path,
        Some(admin.clone()),
        Some(payload),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let action = body(created).await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let path = format!("{create_path}/{id}");
    // Work queued while licensed must recheck before issuing credentials or dispatching.
    let queued = store
        .enqueue(admin.actor_id, id, "Manual", &json!({}), None)
        .await
        .unwrap();
    license.0.store(false, Ordering::Relaxed);
    assert!(
        service
            .process_one(&tokio_util::sync::CancellationToken::new())
            .await
            .unwrap()
    );
    let finished = store.get_run(id, queued.id).await.unwrap();
    assert_eq!(
        finished.status,
        citadel_automation::AutomationRunStatus::Failed
    );
    assert_eq!(
        finished.error_message.as_deref(),
        Some("Automated operations require an active license entitlement.")
    );
    assert!(finished.executor_id.is_none());

    for operation in ["run", "test"] {
        let denied = body(
            request(
                app,
                Method::POST,
                &format!("{path}/{operation}"),
                Some(admin.clone()),
                Some(json!({})),
            )
            .await,
        )
        .await;
        assert_eq!(denied[0]["error"]["code"], 403);
    }
    assert_eq!(store.list_runs(id, 10).await.unwrap().len(), 1);
    // Downgrade retains configuration and allows editing and moving to free placement.
    for patch in [
        json!({"description":"still editable"}),
        json!({"executor":{"kind":"Core"}}),
        json!({"executor":{"kind":"Agent","agentId":Uuid::now_v7()}}),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &path, Some(admin.clone()), Some(patch))
                .await
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"executor":{"kind":"Automatic"}}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    license.0.store(true, Ordering::Relaxed);
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"executor":{"kind":"Automatic"}}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    for operation in ["run", "test"] {
        license.0.store(true, Ordering::Relaxed);
        let accepted = request(
            app,
            Method::POST,
            &format!("{path}/{operation}"),
            Some(admin.clone()),
            Some(json!({})),
        )
        .await;
        assert_eq!(accepted.status(), StatusCode::OK);
        let queued = store
            .list_runs(id, 10)
            .await
            .unwrap()
            .into_iter()
            .find(|run| run.status == citadel_automation::AutomationRunStatus::Queued)
            .expect("Team permits manual and test admission with Automatic placement");
        drop(accepted);
        license.0.store(false, Ordering::Relaxed);
        service.cancel(id, queued.id).await.unwrap();
        assert_eq!(
            store.get_run(id, queued.id).await.unwrap().status,
            citadel_automation::AutomationRunStatus::Cancelled
        );
    }
}
