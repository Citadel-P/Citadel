use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PlatformId")]
    pub platform_id: Uuid,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Spec")]
    pub spec: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentResultActivitySnapshot {
    #[serde(rename = "ContainerIds")]
    pub container_ids: Option<Vec<String>>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "ResourceBindings")]
    pub resource_bindings: Option<Value>,
}
