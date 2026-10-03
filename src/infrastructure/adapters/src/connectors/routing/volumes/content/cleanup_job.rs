use super::*;
use sqlx::Row;

impl VolumeContentAdapter {
    /// Reconcile helpers left Created when Create succeeded but Start/cleanup
    /// could not reach the owning node. Never start or retarget an orphan.
    pub async fn reap_expired(
        &self,
        after: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<Uuid, RuntimeCapabilityError> {
        let rows = sqlx::query("SELECT id,platformid,dockercontainerid,dockernodeid FROM containers WHERE issystem AND systemrole='volume-helper' AND id>$1 AND created<$2 ORDER BY id LIMIT 25")
            .bind(after).bind(chrono::Utc::now().timestamp()-1800).fetch_all(&self.pool).await
            .map_err(|_|error(RuntimeErrorKind::Remote,"Could not read expired Volume helpers."))?;
        let mut last = Uuid::nil();
        for row in rows {
            if cancellation.is_cancelled() {
                break;
            }
            last = row.get("id");
            let target = ContainerTarget {
                id: last,
                platform_id: row.get("platformid"),
                docker_id: row.get("dockercontainerid"),
                node_id: row.get("dockernodeid"),
            };
            let Ok(permit) = self.slots.clone().try_acquire_owned() else {
                break;
            };
            let cancel = cancellation.child_token();
            let _guard = cancel.clone().drop_guard();
            let result = tokio::time::timeout(Duration::from_secs(10),async {
                let (runtime,id,name,labels) = match self.router.resolve(&target,&cancel).await? {
                    Runtime::Local(runtime) => {
                        let value = runtime.inspect_container(&target.docker_id).await.map_err(crate::connectors::docker::runtime::normalize_docker_error)?;
                        (HelperRuntime::Local(runtime.clone()),value.id,value.name,value.config.labels.into_iter().collect::<std::collections::BTreeMap<_,_>>())
                    },
                    Runtime::Agent(runtime) => {
                        let value = runtime.inspect_container(&target.docker_id,&cancel).await?;
                        (HelperRuntime::Agent(AgentExecutionClient::Direct(Arc::new(runtime))),value.id,value.name.unwrap_or_default(),value.config.map(|config|config.labels.into_iter().collect()).unwrap_or_default())
                    },
                    Runtime::Edge(runtime) => {
                        let peer = AgentExecutionClient::Edge(runtime.session);
                        let value:citadel_contracts::citadel::shared_models::v1::InspectContainerResponse = match &peer {
                            AgentExecutionClient::Edge(session)=>crate::connectors::agent::execution::unary(session,EdgeCommandKind::ContainerInspect,
                                citadel_contracts::citadel::containers::v1::InspectContainerRequest { container_id:target.docker_id.clone() },&cancel).await?,
                            _=>unreachable!(),
                        };
                        (HelperRuntime::Agent(peer),value.id,value.name.unwrap_or_default(),value.config.map(|config|config.labels.into_iter().collect()).unwrap_or_default())
                    },
                };
                if id==target.docker_id && expired_owned_helper(target.platform_id,&name,&labels,chrono::Utc::now().timestamp()) {
                    cleanup(runtime,id,permit).await;
                }
                Ok::<_,RuntimeCapabilityError>(())
            }).await;
            if !matches!(result, Ok(Ok(()))) {
                tracing::debug!(platform_id=%target.platform_id,"Expired Volume helper remains pending on its owning node");
            }
        }
        Ok(last)
    }
}

fn expired_owned_helper(
    platform: Uuid,
    name: &str,
    labels: &std::collections::BTreeMap<String, String>,
    now: i64,
) -> bool {
    name.trim_start_matches('/')
        .strip_prefix("citadel-volume-helper-")
        .is_some_and(|suffix| Uuid::parse_str(suffix).is_ok())
        && labels
            .get("com.citadel.system")
            .is_some_and(|value| value == "true")
        && labels
            .get("com.citadel.system-role")
            .is_some_and(|value| value == "volume-helper")
        && labels
            .get("com.citadel.platform-id")
            .is_some_and(|value| value == &platform.to_string())
        && labels
            .get("com.citadel.helper-expires-at")
            .and_then(|value| value.parse::<i64>().ok())
            .is_some_and(|expiry| expiry > 0 && expiry < now)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_requires_exact_ownership_identity_and_expiry() {
        let platform = Uuid::now_v7();
        let name = format!("citadel-volume-helper-{}", Uuid::now_v7().simple());
        let request = helper_request(platform, "data", &name, "helper", HELPER_BINARY);
        let mut labels = request
            .labels
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let now = chrono::Utc::now().timestamp();
        assert!(!expired_owned_helper(platform, &name, &labels, now));
        labels.insert(
            "com.citadel.helper-expires-at".into(),
            (now - 1).to_string(),
        );
        assert!(expired_owned_helper(platform, &name, &labels, now));
        assert!(!expired_owned_helper(Uuid::now_v7(), &name, &labels, now));
        assert!(!expired_owned_helper(
            platform,
            "user-container",
            &labels,
            now
        ));
        for key in [
            "com.citadel.system",
            "com.citadel.system-role",
            "com.citadel.platform-id",
            "com.citadel.helper-expires-at",
        ] {
            let mut wrong = labels.clone();
            wrong.insert(key.into(), "invalid".into());
            assert!(!expired_owned_helper(platform, &name, &wrong, now));
        }
    }
}
