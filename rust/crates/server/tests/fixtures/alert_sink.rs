use citadel_alerts::{AlertError, AlertEventSink, AlertEventView, AlertObservation};
use futures_util::future::BoxFuture;
use std::sync::Mutex;

#[derive(Default)]
pub struct RecordedAlerts(pub Mutex<Vec<AlertObservation>>);
impl AlertEventSink for RecordedAlerts {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>> {
        Box::pin(async move {
            let mut observations = self.0.lock().unwrap();
            assert!(
                observations.len() < 256,
                "Unexpected unbounded producer loop"
            );
            observations.push(observation.clone());
            Ok(None)
        })
    }
}
