use serde_json::Value;

/// Redact registry snapshots before persistence or presentation, including acronym casing.
pub fn mask_registry_credentials(configuration: &mut Value) {
    if let Some(object) = configuration.as_object_mut() {
        for (key, value) in object {
            if ["pat", "password", "accessKey", "secretAccessKey"]
                .iter()
                .any(|secret| key.eq_ignore_ascii_case(secret))
            {
                *value = if value.as_str().is_some_and(|value| !value.is_empty()) {
                    Value::String("******".to_owned())
                } else {
                    Value::Null
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn masks_all_credential_spellings_without_touching_public_settings() {
        let mut config = json!({"pat":"s1","PAT":"s2","pAT":"s3","password":"s4","Password":"s5","accessKey":"s6","SecretAccessKey":"s7","userName":"operator","authEnabled":true});
        mask_registry_credentials(&mut config);
        for key in [
            "pat",
            "PAT",
            "pAT",
            "password",
            "Password",
            "accessKey",
            "SecretAccessKey",
        ] {
            assert_eq!(config[key], "******");
        }
        assert_eq!(config["userName"], "operator");
        assert_eq!(config["authEnabled"], true);
    }
}
