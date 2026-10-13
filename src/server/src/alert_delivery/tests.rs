use super::*;
use crate::api::resources::alerts::views::AlertEventView;
use chrono::Utc;
use serde_json::json;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct Capture(Mutex<Vec<AlertEvent>>);
impl AlertDelivery for Capture {
    fn send<'a>(
        &'a self,
        _: &'a AlertChannel,
        event: &'a AlertEvent,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(event.clone());
            Ok(())
        })
    }
}

#[tokio::test]
async fn notifications_and_api_share_resource_links_without_modifying_stored_messages() {
    let capture = Arc::new(Capture::default());
    let delivery = LinkedAlertDelivery::new(capture.clone(), "https://citadel.example/".into());
    let channel = AlertChannel {
        id: Uuid::now_v7(),
        name: "Test".into(),
        alert_destination: "Generic".into(),
        url: "unused".into(),
        is_active: true,
        audit: citadel_primitives::AuditMetadata {
            created_at: Utc::now(),
            created_by_actor_id: citadel_primitives::ActorId::new(Uuid::from_u128(1)),
        },
    };
    let resource = Uuid::now_v7();
    let run = Uuid::now_v7();
    for (kind, alert_type, info, path) in [
        (
            "BackupPolicy",
            "BackupRunFailed",
            json!({"RunId":run}),
            Some(format!("/backup-policies/edit/{resource}?runId={run}#runs")),
        ),
        (
            "Deployment",
            "DeploymentAutoDeployFailed",
            json!({}),
            Some(format!("/deployments/edit/{resource}")),
        ),
        (
            "Platform",
            "PlatformUnreachable",
            json!({"ResourcePath":"https://untrusted.example/"}),
            Some(format!("/platforms/edit/{resource}")),
        ),
        (
            "BackupPolicy",
            "BackupRunFailed",
            json!({"RunId":"not-a-uuid"}),
            Some(format!("/backup-policies/edit/{resource}")),
        ),
        ("Webhook", "WebhookDispatchFailed", json!({}), None),
    ] {
        let event = AlertEvent {
            id: Uuid::now_v7(),
            alert_rule_id: Uuid::now_v7(),
            alert_type: alert_type.into(),
            severity: citadel_alerts::AlertSeverity::Critical,
            status: citadel_alerts::AlertEventStatus::Active,
            message: "Failure details".into(),
            info,
            resource_id: Some(resource),
            resource_name: "Resource".into(),
            resource_type: kind.into(),
            acknowledged_by_actor_id: None,
            acknowledged_at: None,
            resolved_by_actor_id: None,
            resolved_at: None,
            actor_id: None,
            actor_name: None,
            actor_type: None,
            resolution_note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(
            AlertEventView::try_from(event.clone())
                .unwrap()
                .resource_path,
            path
        );
        delivery
            .send(&channel, &event, &CancellationToken::new())
            .await
            .unwrap();
        let events = capture.0.lock().unwrap();
        let sent = events.last().unwrap();
        assert_eq!(
            sent.message,
            match path {
                Some(path) => format!("Failure details\nhttps://citadel.example{path}"),
                None => "Failure details".into(),
            }
        );
        assert_eq!(sent.info, event.info);
        assert_eq!(event.message, "Failure details");
    }
}
