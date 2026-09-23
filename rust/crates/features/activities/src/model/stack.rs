use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "StackSource")]
    pub stack_source: String,
    #[serde(rename = "DriftPolicy")]
    pub drift_policy: Value,
    #[serde(rename = "StackRelease")]
    pub stack_release: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StackResultActivitySnapshot {
    #[serde(rename = "ContainerIds")]
    pub container_ids: Option<Vec<String>>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "ResourceBindings")]
    pub resource_bindings: Option<Value>,
}
