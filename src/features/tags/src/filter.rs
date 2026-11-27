use crate::{TagError, validate_tag_name};

/// Space for a selector when represented as repeated URL-encoded parameters.
/// Leave room for route and platform constraints in the HTTP request target.
pub const MAX_TAG_FILTER_BYTES: usize = 4096;

pub fn tag_filter_names<I, S>(values: I) -> Result<Vec<String>, TagError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut names = Vec::new();
    let mut encoded_bytes = 0;
    for value in values {
        let name = value.as_ref().trim();
        validate_tag_name(name)?;
        if names.len() >= 100 {
            return Err(TagError::Validation(
                "At most 100 tags may be filtered at once.".into(),
            ));
        }
        encoded_bytes += 5
            + usize::from(!names.is_empty())
            + name
                .bytes()
                .map(|byte| {
                    if byte.is_ascii_alphanumeric()
                        || matches!(byte, b'*' | b'-' | b'.' | b'_' | b' ')
                    {
                        1usize
                    } else {
                        3
                    }
                })
                .sum::<usize>();
        if encoded_bytes > MAX_TAG_FILTER_BYTES {
            return Err(TagError::Validation(
                "Tag filter exceeds 4096 encoded bytes.".into(),
            ));
        }
        names.push(name.to_owned());
    }
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_unicode_and_literal_names_with_an_aggregate_budget() {
        let name = "à".repeat(128);
        assert_eq!(
            tag_filter_names([&name]).unwrap().as_slice(),
            std::slice::from_ref(&name)
        );
        assert!(tag_filter_names([&name; 6]).is_err());
        assert_eq!(
            tag_filter_names([" x:y,+&%_ ", "x:y,+&%_"]).unwrap(),
            ["x:y,+&%_"]
        );
        assert!(tag_filter_names([" "]).is_err());
    }
}
