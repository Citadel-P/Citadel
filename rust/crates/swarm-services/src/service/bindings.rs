use super::*;

pub(super) fn normalize_name(value: &mut String) -> Result<(), SwarmServiceError> {
    *value = value.trim().to_owned();
    if value.is_empty() || value.len() > 100 {
        return Err(validation(
            "Service name must contain between 1 and 100 characters.",
        ));
    }
    Ok(())
}
pub(super) fn normalize_description(value: &mut Option<String>) -> Result<(), SwarmServiceError> {
    if let Some(description) = value {
        *description = description.trim().to_owned();
        if description.len() > 500 {
            return Err(validation(
                "Service description cannot exceed 500 characters.",
            ));
        }
        if description.is_empty() {
            *value = None;
        }
    }
    Ok(())
}
pub(super) fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut seen = HashSet::with_capacity(ids.len());
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}
pub(super) fn referenced_binding_names(
    environment: &[String],
) -> Result<Vec<String>, SwarmServiceError> {
    let mut names = Vec::new();
    for entry in environment {
        if let Some((name, _)) = entry.split_once('=') {
            if name.trim().is_empty() {
                return Err(validation(
                    "Service environment variable names must not be empty.",
                ));
            }
        } else if valid_binding_name(entry) {
            push_unique_name(&mut names, entry);
        }
        let mut offset = 0;
        while let Some(start) = entry[offset..].find("${") {
            let name_start = offset + start + 2;
            let Some(end) = entry[name_start..].find('}') else {
                return Err(validation(&format!(
                    "Service environment entry '{entry}' has an incomplete variable reference."
                )));
            };
            let name = &entry[name_start..name_start + end];
            if !valid_binding_name(name) {
                return Err(validation(&format!(
                    "Service environment entry '{entry}' contains an invalid variable reference."
                )));
            }
            push_unique_name(&mut names, name);
            offset = name_start + end + 1;
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
pub(super) fn redact(mut message: String, values: &[String]) -> String {
    for value in values.iter().filter(|value| !value.is_empty()) {
        message = message.replace(value, "********");
    }
    message
}
pub(super) fn validation(message: &str) -> SwarmServiceError {
    SwarmServiceError::Validation(message.to_owned())
}
