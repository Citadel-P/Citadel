use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGitAccount {
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGitAccount {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub transport: Option<GitTransport>,
    pub auth_type: Option<GitAuthType>,
    pub configuration: Option<GitAuthConfiguration>,
}

impl CreateGitAccount {
    pub fn validate(&mut self) -> Result<(), GitAccountError> {
        validate_name(&self.name)?;
        validate_domain(&self.domain)?;
        validate_configuration(self.transport, self.auth_type, &self.configuration)?;
        self.name = self.name.trim().to_owned();
        self.domain = self.domain.trim().trim_end_matches('/').to_owned();
        Ok(())
    }
}
