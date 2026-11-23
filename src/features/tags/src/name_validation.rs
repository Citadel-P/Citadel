use crate::*;

pub fn validate_tag_name(name: &str) -> Result<(), TagError> {
    let length = name.trim().chars().count();
    if length == 0 || length > 128 || name.contains('\0') {
        return Err(TagError::Validation(
            "Tag names must contain between 1 and 128 characters and no NUL.".into(),
        ));
    }
    Ok(())
}
