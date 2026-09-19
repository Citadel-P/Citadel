use super::*;

#[derive(Debug, Clone)]
pub struct GitAccountConfiguration {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub transport: GitTransport,
    pub auth_type: GitAuthType,
    pub configuration: GitAuthConfiguration,
}
