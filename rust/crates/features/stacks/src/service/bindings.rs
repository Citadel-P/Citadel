use super::*;

impl StackProgress {
    pub fn new(
        sender: mpsc::Sender<StackProgressItem>,
        secrets: Vec<String>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            sender,
            secrets,
            cancellation,
        }
    }

    pub async fn send(&self, mut item: StackProgressItem) {
        item.message = item
            .message
            .take()
            .map(|message| redact(message, &self.secrets));
        tokio::select! {
            biased;
            () = self.cancellation.cancelled() => {},
            _ = self.sender.send(item) => {},
        }
    }
}

pub(super) const fn state_action_status(action: StackAction) -> StackReleaseStatus {
    match action {
        StackAction::Start | StackAction::Resume | StackAction::Restart => {
            StackReleaseStatus::Healthy
        }
        StackAction::Stop => StackReleaseStatus::Stopped,
        StackAction::Pause => StackReleaseStatus::Paused,
    }
}

pub(super) fn resolve_environment(
    model: &ComposeModel,
    resolved: &ResolvedStackBindings,
) -> Result<(Vec<String>, Vec<String>), StackError> {
    let mut values = BTreeMap::new();
    for binding in &resolved.entries {
        values.insert(binding.name.to_ascii_lowercase(), binding);
    }
    let mut environment = Vec::with_capacity(model.variables.len());
    let mut redactions = Vec::new();
    for name in &model.variables {
        let Some(binding) = values.get(&name.to_ascii_lowercase()) else {
            if model.optional_variables.contains(name) {
                continue;
            }
            return Err(validation(&format!(
                "Stack references undefined Citadel variable or secret '{name}'."
            )));
        };
        environment.push(format!("{name}={}", binding.value.as_str()));
        if binding.secret {
            redactions.push(binding.value.to_string());
        }
    }
    Ok((environment, redactions))
}

pub(super) fn redact(mut message: String, secrets: &[String]) -> String {
    for value in secrets.iter().filter(|value| !value.is_empty()) {
        message = message.replace(value, "********");
    }
    message
}

pub(super) fn redact_runtime_messages(result: &mut StackRuntimeResult, secrets: &[String]) {
    for item in &mut result.messages {
        item.message = item.message.take().map(|message| redact(message, secrets));
        item.severity = item
            .severity
            .take()
            .map(|severity| redact(severity, secrets));
    }
}

pub(super) fn orchestration(
    platform_type: &citadel_platforms::PlatformKind,
) -> StackOrchestrationMode {
    match platform_type {
        citadel_platforms::PlatformKind::Docker => StackOrchestrationMode::DockerCompose,
        citadel_platforms::PlatformKind::DockerSwarm => StackOrchestrationMode::DockerSwarm,
    }
}

pub(super) fn project_name(stack: &StackDetails) -> Result<String, StackError> {
    if let Some(project) = stack
        .spec
        .as_ref()
        .and_then(|spec| spec.common().project_name.clone())
    {
        return Ok(project);
    }
    let normalized = stack
        .name
        .to_ascii_lowercase()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if normalized.is_empty() {
        Ok(format!("stack-{}", stack.id.simple()))
    } else {
        Ok(normalized)
    }
}

#[must_use]
pub fn import_runtime_fingerprint(claim: &StackImportClaim) -> String {
    compose_digest(&[
        claim.platform_id.to_string(),
        claim.project_name.clone(),
        claim
            .orphaned_owner_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        format!("{:?}", claim.import_kind),
        claim.service_names.join("\n"),
        claim.runtime_fingerprint.clone(),
    ])
}
