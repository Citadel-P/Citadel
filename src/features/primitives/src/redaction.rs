/// Shared environment-name policy for inspection redaction and resource adoption.
pub fn is_sensitive_environment_name(name: &str) -> bool {
    let normalized = name.to_ascii_uppercase();
    let parts: Vec<_> = normalized.split(['_', '.', '-']).collect();
    parts.iter().enumerate().any(|(index, part)| {
        matches!(
            *part,
            "PASSWORD"
                | "PASSWD"
                | "SECRET"
                | "TOKEN"
                | "CREDENTIAL"
                | "CREDENTIALS"
                | "APIKEY"
                | "ACCESSKEY"
                | "PRIVATEKEY"
                | "ENCRYPTIONKEY"
                | "SIGNINGKEY"
                | "CONNECTIONSTRING"
                | "CONNECTIONSTRINGS"
                | "DATABASEURL"
                | "DATABASEDSN"
        ) || parts.get(index + 1).is_some_and(|next| match *part {
            "API" | "ACCESS" | "PRIVATE" | "ENCRYPTION" | "SIGNING" => *next == "KEY",
            "CONNECTION" => matches!(*next, "STRING" | "STRINGS"),
            "DATABASE" => matches!(*next, "URL" | "DSN"),
            _ => false,
        })
    })
}
