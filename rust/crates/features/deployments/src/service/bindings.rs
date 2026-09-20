use super::*;
pub(super) fn referenced_binding_names(
    environment: &[String],
) -> Result<Vec<String>, DeploymentError> {
    let mut names = Vec::new();
    for entry in environment {
        if let Some((name, _)) = entry.split_once('=') {
            if name.trim().is_empty() {
                return Err(DeploymentError::Validation(
                    "Deployment environment variable names must not be empty.".to_owned(),
                ));
            }
        } else if valid_binding_name(entry) {
            push_unique_name(&mut names, entry);
        }
        let bytes = entry.as_bytes();
        let mut offset = 0;
        while let Some(start) = entry[offset..].find("${") {
            let name_start = offset + start + 2;
            let Some(end) = entry[name_start..].find('}') else {
                return Err(DeploymentError::Validation(format!(
                    "Deployment environment entry '{entry}' has an incomplete variable reference."
                )));
            };
            let name = &entry[name_start..name_start + end];
            if !valid_binding_name(name) {
                return Err(DeploymentError::Validation(format!(
                    "Deployment environment entry '{entry}' contains an invalid variable reference."
                )));
            }
            push_unique_name(&mut names, name);
            offset = name_start + end + 1;
            if offset >= bytes.len() {
                break;
            }
        }
    }
    Ok(names)
}

pub(super) fn valid_binding_name(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

pub(super) fn push_unique_name(names: &mut Vec<String>, value: &str) {
    if !names.iter().any(|name| name.eq_ignore_ascii_case(value)) {
        names.push(value.to_owned());
    }
}

pub(super) struct ResolvedDeploymentEnvironment {
    pub(super) values: Vec<String>,
    pub(super) snapshots: Vec<DeploymentBindingSnapshot>,
    pub(super) redaction_values: Vec<String>,
}

pub(super) fn build_environment(
    configured: &[String],
    referenced: &[String],
    resolved: &ResolvedDeploymentBindings,
) -> Result<ResolvedDeploymentEnvironment, DeploymentError> {
    let selected = resolved
        .entries
        .iter()
        .filter(|entry| {
            referenced
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&entry.name))
        })
        .collect::<Vec<_>>();
    for name in referenced {
        if !selected
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(name))
        {
            return Err(DeploymentError::Validation(format!(
                "Deployment references undefined Citadel variable or secret '{name}'."
            )));
        }
    }
    let mut output = Vec::with_capacity(configured.len());
    for configured_entry in configured {
        if !configured_entry.contains('=') && valid_binding_name(configured_entry) {
            let binding = selected
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(configured_entry))
                .expect("referenced binding was validated");
            output.push(format!("{configured_entry}={}", binding.value.as_str()));
            continue;
        }
        let mut value = configured_entry.clone();
        for binding in &selected {
            value = value.replace(&format!("${{{}}}", binding.name), binding.value.as_str());
        }
        output.push(value);
    }
    let snapshots = selected
        .iter()
        .map(|entry| entry.snapshot.clone())
        .collect();
    let redaction_values = selected
        .iter()
        .filter(|entry| entry.secret)
        .map(|entry| entry.value.to_string())
        .collect();
    Ok(ResolvedDeploymentEnvironment {
        values: output,
        snapshots,
        redaction_values,
    })
}

pub(super) fn binding_message(resolved: &ResolvedDeploymentBindings) -> String {
    let variables = resolved
        .entries
        .iter()
        .filter(|entry| !entry.secret)
        .count();
    let secrets = resolved.entries.iter().filter(|entry| entry.secret).count();
    if variables == 0 && secrets == 0 {
        "No Citadel variables or secrets were referenced by this Deployment.".to_owned()
    } else {
        format!("Injected {variables} Citadel variable(s) and {secrets} secret(s).")
    }
}

pub(super) fn redact_error(mut error: DeploymentError, values: &[String]) -> DeploymentError {
    // Redact the underlying detail, preserving the error kind and its one prefix.
    // Formatting and rewrapping a Runtime error duplicates its display prefix.
    match &mut error {
        DeploymentError::Validation(message)
        | DeploymentError::Conflict(message)
        | DeploymentError::Runtime(message)
        | DeploymentError::Storage(message) => {
            for value in values.iter().filter(|value| !value.is_empty()) {
                *message = message.replace(value, "********");
            }
        }
        DeploymentError::NotFound
        | DeploymentError::Forbidden
        | DeploymentError::LicenseRequired(_)
        | DeploymentError::Cancelled => {}
    }
    error
}
