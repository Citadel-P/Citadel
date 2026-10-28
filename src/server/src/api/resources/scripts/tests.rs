use super::{spec::*, views::*};
use serde_json::json;
use uuid::Uuid;

fn run() -> citadel_scripts::ScriptRun {
    citadel_scripts::ScriptRun {
        executor: Default::default(),
        attempt_id: None,
        executor_id: None,
        cleanup_pending: false,
        id: Uuid::now_v7(),
        script_id: Uuid::now_v7(),
        script_name: "maintenance".into(),
        trigger: "Manual".into(),
        status: citadel_scripts::ScriptRunStatus::Failed,
        run_as_actor_id: Uuid::now_v7(),
        triggered_by_actor_id: None,
        args_json: "{}".into(),
        code_snapshot: None,
        code_hash: "hash".into(),
        timeout_seconds: 300,
        queued_at: chrono::Utc::now(),
        started_at: None,
        finished_at: None,
        duration_ms: None,
        exit_code: Some(7),
        logs: Some("failure output".into()),
        error_message: Some("Execution failed".into()),
    }
}

#[test]
fn run_vocabulary_matches_execution_and_scheduler_states() {
    for status in [
        "Queued",
        "Running",
        "Succeeded",
        "Failed",
        "TimedOut",
        "Cancelled",
        "Rejected",
    ] {
        assert!(serde_json::from_value::<ScriptRunStatus>(json!(status)).is_ok());
    }
    for trigger in ["Manual", "Test", "Schedule", "Webhook"] {
        assert!(serde_json::from_value::<ScriptRunTrigger>(json!(trigger)).is_ok());
    }
    assert!(serde_json::from_value::<ScriptRunStatus>(json!("Unknown")).is_err());
    assert!(serde_json::from_value::<ScriptRunTrigger>(json!("Unknown")).is_err());
}

#[test]
fn run_view_preserves_diagnostics_and_rejects_invalid_persisted_vocabulary() {
    let mut run = run();
    let view = ScriptRunView::try_from(run.clone()).unwrap();
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["status"], "Failed");
    assert_eq!(wire["trigger"], "Manual");
    assert_eq!(wire["errorMessage"], "Execution failed");
    assert_eq!(wire["logs"], "failure output");
    assert_eq!(wire["exitCode"], 7);
    assert!(wire["codeSnapshot"].is_null());
    run.trigger = "Unexpected".into();
    assert!(ScriptRunView::try_from(run.clone()).is_err());
    assert!(
        "Unexpected"
            .parse::<citadel_scripts::ScriptRunStatus>()
            .is_err()
    );
}

#[test]
fn progress_preserves_terminal_errors_and_output_only_items() {
    let run = run();
    let result = citadel_scripts::ScriptRunResult {
        status: citadel_scripts::ScriptRunStatus::Failed,
        exit_code: Some(7),
        logs: "failure output".into(),
        error: Some("Execution failed".into()),
    };
    let terminal = citadel_scripts::ScriptProgress::completed(&run, &result);
    let wire = serde_json::to_value(ScriptProgress::try_from(terminal).unwrap()).unwrap();
    assert_eq!(wire["status"], "Failed");
    assert_eq!(wire["runId"], run.id.to_string());
    assert_eq!(
        wire["error"],
        json!({"code":500,"message":"Execution failed"})
    );
    let output = citadel_scripts::ScriptProgress {
        stream: Some("stdout".into()),
        progress_message: Some("output".into()),
        ..Default::default()
    };
    let wire = serde_json::to_value(ScriptProgress::try_from(output).unwrap()).unwrap();
    assert_eq!(wire["progressMessage"], "output");
    assert!(wire["status"].is_null());
    assert!(wire["error"].is_null());
}

#[test]
fn script_endpoints_derive_native_run_and_progress_schemas() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    let detail = &doc["paths"]["/api/v1/scripts/{id}/runs/{runId}"]["get"]["responses"]["200"]["content"]
        ["application/json"]["schema"];
    assert_eq!(detail["$ref"], "#/components/schemas/ScriptRunView");
    for operation in ["run", "test"] {
        let path = format!("/api/v1/scripts/{{id}}/{operation}");
        let stream = &doc["paths"][path]["post"]["responses"]["200"]["content"]["application/json"]
            ["schema"];
        assert_eq!(
            stream["items"]["$ref"],
            "#/components/schemas/ScriptProgress"
        );
    }
    assert!(
        schemas["ScriptProgress"]["properties"]["status"]
            .to_string()
            .contains("#/components/schemas/ScriptRunStatus")
    );
    assert_eq!(
        schemas["ScriptRunView"]["properties"]["trigger"]["$ref"],
        "#/components/schemas/ScriptRunTrigger"
    );
}
