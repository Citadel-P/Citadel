use serde_json::Value;

/// Citadel's stored/public property convention changes only the first ASCII letter.
/// Remaining letters, Unicode, and discriminator keys such as `$type` are left intact.
#[derive(Clone, Copy)]
pub enum PropertyCase {
    Camel,
    Pascal,
}

impl PropertyCase {
    #[must_use]
    pub fn key(self, mut key: String) -> String {
        if let Some(first) = key.get_mut(..1) {
            match self {
                Self::Camel => first.make_ascii_lowercase(),
                Self::Pascal => first.make_ascii_uppercase(),
            }
        }
        key
    }
}

/// Convert structured property names, leaving explicitly named dictionary objects
/// untouched. Dictionary names are matched case-insensitively before conversion.
/// Array values still contain structured records and are traversed normally.
#[must_use]
pub fn map_property_keys(value: Value, case: PropertyCase, dictionaries: &[&str]) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let opaque = value.is_object()
                        && dictionaries
                            .iter()
                            .any(|name| key.eq_ignore_ascii_case(name));
                    let value = if opaque {
                        value
                    } else {
                        map_property_keys(value, case, dictionaries)
                    };
                    (case.key(key), value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| map_property_keys(value, case, dictionaries))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_records_but_preserves_dictionary_subtrees_and_values() {
        let public = json!({"$type":"Git", "items":[{"imageId":"ABC", "labels":{"Owner":"OPS","nested":{"KeepCase":true}}}], "environmentVariables":{"PATH":"/bin"}, "équipe":"OPS", "":null});
        let dictionaries = ["labels", "environmentVariables"];
        let stored = map_property_keys(public.clone(), PropertyCase::Pascal, &dictionaries);
        assert_eq!(stored["Items"][0]["ImageId"], "ABC");
        assert_eq!(stored["Items"][0]["Labels"], public["items"][0]["labels"]);
        assert_eq!(stored["EnvironmentVariables"]["PATH"], "/bin");
        assert_eq!(stored["$type"], "Git");
        assert_eq!(
            map_property_keys(stored, PropertyCase::Camel, &dictionaries),
            public
        );
    }

    #[test]
    fn dictionary_names_do_not_stop_record_array_conversion() {
        assert_eq!(
            map_property_keys(
                json!({"BuildArgs":[{"Name":"ARG","Value":"KeepCase"}]}),
                PropertyCase::Camel,
                &["buildArgs"]
            ),
            json!({"buildArgs":[{"name":"ARG","value":"KeepCase"}]})
        );
        assert_eq!(PropertyCase::Camel.key("URL".into()), "uRL");
    }
}
