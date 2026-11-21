use serde_json::Value;

/// Borrow credentials from the stored configuration so selection makes no secret copies.
pub struct RegistryConfiguration<'a>(pub &'a Value);
impl<'a> RegistryConfiguration<'a> {
    pub fn kind(&self) -> &'a str {
        self.0
            .get("$type")
            .or_else(|| self.0.get("type"))
            .and_then(Value::as_str)
            .unwrap_or_default()
    }
    fn field(&self, name: &str) -> Option<&'a Value> {
        self.0
            .as_object()?
            .iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case(name).then_some(value))
    }
    pub fn string(&self, name: &str) -> Option<&'a str> {
        self.field(name)?.as_str()
    }
    pub fn auth_enabled(&self) -> bool {
        match self.kind() {
            "Custom" => self.field("authEnabled").and_then(Value::as_bool) == Some(true),
            "GitHub" => self.field("ghcrAuthEnabled").and_then(Value::as_bool) == Some(true),
            "DockerHub" | "Azure" | "Gitlab" => true,
            _ => false,
        }
    }
    pub fn user_password(&self) -> Option<(&'a str, &'a str)> {
        if !self.auth_enabled() {
            return None;
        }
        let (user, secret) = match self.kind() {
            "Custom" | "Azure" => ("userName", "password"),
            "DockerHub" | "Gitlab" => ("userName", "pat"),
            "GitHub" => ("nameSpace", "pat"),
            _ => return None,
        };
        self.string(user)
            .filter(|s| !s.is_empty())
            .zip(self.string(secret).filter(|s| !s.is_empty()))
    }
}

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
    fn stored_secrets_follow_provider_opt_in() {
        for (kind, flag) in [("GitHub", "ghcrAuthEnabled"), ("Custom", "authEnabled")] {
            let mut value = json!({"$type":kind,"userName":"user","nameSpace":"user","pat":"secret","password":"secret"});
            for setting in [None, Some(false), Some(true)] {
                if let Some(setting) = setting {
                    value[flag] = json!(setting);
                }
                assert_eq!(
                    RegistryConfiguration(&value).user_password(),
                    setting.filter(|v| *v).map(|_| ("user", "secret"))
                );
            }
            value["pat"] = Value::Null;
            value["password"] = json!("");
            assert!(RegistryConfiguration(&value).user_password().is_none());
        }
        for kind in ["DockerHub", "Azure", "Gitlab", "AWS"] {
            let value = json!({"$type":kind,"UserName":"user","Password":"secret","PAT":"secret"});
            assert_eq!(
                RegistryConfiguration(&value).user_password(),
                (kind != "AWS").then_some(("user", "secret"))
            );
        }
    }

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
