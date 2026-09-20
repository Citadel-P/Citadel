use super::*;

pub(super) fn validate_name(name: &str) -> Result<(), GitAccountError> {
    let name = name.trim();
    if !(3..=64).contains(&name.len())
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(GitAccountError::Validation(
            "Git account name must contain 3 to 64 letters, numbers, hyphens, or underscores."
                .to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn validate_domain(domain: &str) -> Result<(), GitAccountError> {
    let domain = domain.trim().trim_end_matches('/');
    let valid = !domain.is_empty()
        && domain.len() <= 253
        && !domain.contains(['/', ':', '@', ' ', '\r', '\n'])
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        });
    if valid {
        Ok(())
    } else {
        Err(GitAccountError::Validation(
            "Please provide a valid host name, for example github.com.".to_owned(),
        ))
    }
}

pub(super) fn validate_configuration(
    transport: GitTransport,
    auth_type: GitAuthType,
    configuration: &GitAuthConfiguration,
) -> Result<(), GitAccountError> {
    let matches = matches!(
        (auth_type, configuration),
        (GitAuthType::Basic, GitAuthConfiguration::Basic { .. })
            | (GitAuthType::Token, GitAuthConfiguration::Token { .. })
            | (GitAuthType::SshKey, GitAuthConfiguration::SshKey { .. })
    );
    if !matches {
        return Err(GitAccountError::Validation(
            "Git authentication type does not match its configuration.".to_owned(),
        ));
    }
    match (transport, configuration) {
        (GitTransport::Ssh, GitAuthConfiguration::SshKey { .. })
        | (GitTransport::Http | GitTransport::Https, GitAuthConfiguration::Basic { .. })
        | (GitTransport::Http | GitTransport::Https, GitAuthConfiguration::Token { .. }) => {}
        (GitTransport::Ssh, _) => {
            return Err(GitAccountError::Validation(
                "SSH requires SSH key authentication.".to_owned(),
            ));
        }
        (_, GitAuthConfiguration::SshKey { .. }) => {
            return Err(GitAccountError::Validation(
                "SSH authentication cannot be used with HTTP or HTTPS.".to_owned(),
            ));
        }
    }
    let populated = match configuration {
        GitAuthConfiguration::Basic { username, password } => {
            !username.trim().is_empty() && !password.is_empty()
        }
        GitAuthConfiguration::Token { token } => !token.is_empty(),
        GitAuthConfiguration::SshKey {
            username,
            private_key,
            ..
        } => !username.trim().is_empty() && !private_key.trim().is_empty(),
    };
    if populated {
        Ok(())
    } else {
        Err(GitAccountError::Validation(
            "Git credentials cannot be empty.".to_owned(),
        ))
    }
}
