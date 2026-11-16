use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "Id":"fixture", "RestartCount":3,
        "Config":{"Healthcheck":{"Test":["CMD-SHELL","probe"]},"Env":["PASSWORD=private"]},
        "State":{"Status":"running","ExitCode":0,"OOMKilled":false,
            "StartedAt":"2026-11-16T10:00:00Z","FinishedAt":"0001-01-01T00:00:00Z",
            "Health":{"Status":"healthy","FailingStreak":0,"Log":[
                {"End":"2026-11-16T10:00:01Z","Output":"old output"},
                {"End":"2026-11-16T10:00:02Z","Output":"private output"}]}}
    })
}

fn assert_equivalent(doc: Value) {
    let bytes = serde_json::to_vec(&doc).unwrap();
    let actual = parse(&bytes).unwrap();
    let mut expected = super::super::container_health::observation(&doc);
    // Collection time is the only deliberately nondeterministic field.
    expected.observed_at = actual.health.observed_at;
    assert_eq!(
        serde_json::to_value(actual.health).unwrap(),
        serde_json::to_value(expected).unwrap(),
        "{doc}"
    );
    assert_eq!(actual.id.as_deref(), doc["Id"].as_str());
    assert_eq!(
        actual.state,
        doc["State"]["Status"].as_str().unwrap_or("unknown")
    );
}

#[test]
fn projection_preserves_health_evidence_and_missing_values() {
    for doc in [
        fixture(),
        json!({}),
        json!({"Config":{}}),
        json!({"Config":null,"State":null}),
        json!({"Config":{"Healthcheck":{"Test":["NONE"]}},"State":{"Status":"exited","ExitCode":137}}),
        json!({"Config":{"Healthcheck":{"Test":[]}}}),
        json!({"Config":{"Healthcheck":{"Test":["future-kind"]}}}),
    ] {
        assert_equivalent(doc);
    }
    for status in ["healthy", "unhealthy", "starting", "unknown"] {
        let mut doc = fixture();
        doc["State"]["Health"]["Status"] = json!(status);
        doc["State"]["OOMKilled"] = json!(true);
        doc["State"]["ExitCode"] = json!(137);
        doc["State"]["Health"]["FailingStreak"] = json!(4);
        assert_equivalent(doc);
    }
    for last in [
        json!({}),
        json!({"End":"invalid"}),
        json!({"End":"1970-01-01T00:00:00Z"}),
    ] {
        let mut doc = fixture();
        doc["State"]["Health"]["Log"][1] = last;
        doc["RestartCount"] = json!(-1);
        doc["State"]["Health"]["FailingStreak"] = json!(-1);
        assert_equivalent(doc);
    }
}

#[test]
fn unexpected_types_keep_permissive_evidence_without_accepting_invalid_json() {
    for (pointer, value) in [
        ("/Config", json!([])),
        ("/Config/Healthcheck/Test", json!([123, "CMD"])),
        ("/State/Health/Log/1", json!(false)),
        ("/State/Health/FailingStreak", json!("4")),
        ("/State/Health/Status", json!(true)),
        ("/RestartCount", json!(18446744073709551615_u64)),
        ("/State/ExitCode", json!(1.5)),
        ("/Id", json!(1)),
    ] {
        let mut doc = fixture();
        *doc.pointer_mut(pointer).unwrap() = value;
        assert_equivalent(doc);
    }
    assert!(parse(b"{invalid").is_err());
}
