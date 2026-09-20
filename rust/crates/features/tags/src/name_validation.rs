use crate::*;

pub(crate) fn validate_name(name: &str, resource: &str) -> Result<(), TagError> {
    let length = name.trim().chars().count();
    if length == 0 || length > 128 {
        return Err(TagError::Validation(format!(
            "{resource} name must contain between 1 and 128 characters."
        )));
    }
    Ok(())
}
