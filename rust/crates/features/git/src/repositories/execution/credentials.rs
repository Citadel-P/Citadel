use super::*;
impl GitRepositoryExecutionService {
    pub(super) async fn prepare_remote(
        &self,
        source: &GitRepositorySource,
    ) -> Result<PreparedRemote, GitRepositoryExecutionError> {
        let Some(account_id) = source.git_account_id else {
            return Ok(PreparedRemote::new(normalize_remote_url(
                &source.url,
                None,
            )?));
        };
        let account = self
            .accounts
            .get_config(account_id)
            .await
            .map_err(|_| GitRepositoryExecutionError::Credential)?;
        let ssh_username = match &account.configuration {
            GitAuthConfiguration::SshKey { username, .. } => Some(username.as_str()),
            _ => None,
        };
        let url = normalize_remote_url(
            &source.url,
            Some((&account.domain, account.transport, ssh_username)),
        )?;
        let mut remote = PreparedRemote::new(url);
        match account.configuration {
            GitAuthConfiguration::Basic { username, password } => {
                remote.add_basic_header(&username, &password)?;
            }
            GitAuthConfiguration::Token { token } => {
                remote.add_basic_header("git", &token)?;
            }
            GitAuthConfiguration::SshKey {
                private_key,
                passphrase,
                ..
            } => {
                if passphrase.as_deref().is_some_and(|value| !value.is_empty()) {
                    return Err(GitRepositoryExecutionError::Validation(
                        "Passphrase-protected SSH keys are not supported by unattended Git synchronization."
                            .to_owned(),
                    ));
                }
                remote
                    .add_ssh_key(self.cli.workspace.as_ref(), &self.cache_root, &private_key)
                    .await?;
            }
        }
        Ok(remote)
    }
}

pub(super) struct PreparedRemote {
    pub(super) url: String,
    pub(super) environment: Vec<(OsString, OsString)>,
    pub(super) credential_file: Option<Box<dyn crate::workspace::GitCredentialFile>>,
}

impl PreparedRemote {
    fn new(url: String) -> Self {
        Self {
            url,
            environment: Vec::new(),
            credential_file: None,
        }
    }

    fn add_basic_header(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(), GitRepositoryExecutionError> {
        let url =
            url::Url::parse(&self.url).map_err(|_| GitRepositoryExecutionError::Credential)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(GitRepositoryExecutionError::Credential);
        }
        // Recursive submodules may point to other hosts. Only send this account's
        // credentials to the source origin, including its scheme and port.
        let origin = url.origin().ascii_serialization();
        let encoded = STANDARD.encode(format!("{username}:{password}"));
        self.environment.extend([
            (OsString::from("GIT_CONFIG_COUNT"), OsString::from("1")),
            (
                OsString::from("GIT_CONFIG_KEY_0"),
                OsString::from(format!("http.{origin}/.extraHeader")),
            ),
            (
                OsString::from("GIT_CONFIG_VALUE_0"),
                OsString::from(format!("Authorization: Basic {encoded}")),
            ),
        ]);
        Ok(())
    }

    async fn add_ssh_key(
        &mut self,
        workspace: &dyn crate::workspace::GitWorkspacePort,
        cache_root: &Path,
        private_key: &str,
    ) -> Result<(), GitRepositoryExecutionError> {
        self.credential_file = Some(
            workspace
                .credential(cache_root, private_key)
                .await
                .map_err(storage_error)?,
        );
        let path = self
            .credential_file
            .as_ref()
            .expect("credential created")
            .path();
        let command = format!(
            "ssh -i \"{}\" -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -o BatchMode=yes",
            path.display()
        );
        self.environment
            .push((OsString::from("GIT_SSH_COMMAND"), OsString::from(command)));
        Ok(())
    }
}

pub(super) fn normalize_remote_url(
    url: &str,
    account: Option<(&str, GitTransport, Option<&str>)>,
) -> Result<String, GitRepositoryExecutionError> {
    let url = url.trim();
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("git@")
    {
        return Ok(url.to_owned());
    }
    let Some((domain, transport, username)) = account else {
        return Err(GitRepositoryExecutionError::Validation(
            "A complete repository URL is required when no Git account is selected.".to_owned(),
        ));
    };
    let path = url.trim_start_matches('/');
    let scheme = match transport {
        GitTransport::Http => "http",
        GitTransport::Https => "https",
        GitTransport::Ssh => {
            let username = username
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("git");
            return Ok(format!("{username}@{domain}:{path}"));
        }
    };
    Ok(format!("{scheme}://{domain}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_credentials_match_sibling_repositories_but_not_other_origins() {
        let mut remote = PreparedRemote::new("https://github.com/team/source".into());
        remote.add_basic_header("git", "test-token").unwrap();
        for (url, expected) in [
            ("https://github.com/team/contracts", true),
            ("https://github.com/other/contracts", true),
            ("https://github.com.evil.test/team/contracts", false),
            ("http://github.com/team/contracts", false),
            ("https://github.com:8443/team/contracts", false),
        ] {
            let output = std::process::Command::new("git")
                .args(["config", "--get-urlmatch", "http.extraHeader", url])
                .envs(remote.environment.iter().cloned())
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .output()
                .unwrap();
            assert_eq!(output.status.success(), expected, "{url}");
            if expected {
                assert_eq!(
                    String::from_utf8(output.stdout).unwrap().trim(),
                    format!("Authorization: Basic {}", STANDARD.encode("git:test-token"))
                );
            } else {
                assert!(output.stdout.is_empty());
            }
        }
    }
}
