use std::collections::BTreeMap;

use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{ImageCapabilitiesView, RuntimeCapabilityError};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspection {
    pub id: String,
    pub name: String,
    pub tag: String,
    pub size: i64,
    pub os: String,
    pub created: String,
    pub architecture: String,
    pub env: Vec<String>,
    pub cmd: Vec<String>,
    pub repo_tags: Vec<String>,
    pub volumes: Vec<String>,
    pub exposed_ports: Vec<String>,
    pub layers: Vec<ImageLayer>,
    pub labels: BTreeMap<String, String>,
    pub containers: Vec<ImageContainer>,
    pub registry: Option<Value>,
    pub docker_node_id: Option<String>,
    pub capabilities: Option<ImageCapabilitiesView>,
}

impl ImageInspection {
    pub fn set_display_reference(&mut self) {
        let Some(reference) = self.repo_tags.first() else {
            return;
        };
        if let Some((name, tag)) = reference.rsplit_once(':')
            && !tag.contains('/')
        {
            self.name = name.to_owned();
            self.tag = tag.to_owned();
        } else {
            self.name = reference.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLayer {
    pub id: String,
    pub created: i64,
    pub created_by: String,
    pub size: i64,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContainer {
    pub id: String,
    pub name: String,
    pub state: String,
    pub volumes: Vec<String>,
    pub networks: BTreeMap<String, String>,
    pub ports: Value,
}

pub trait ImageInspectionPort: Send + Sync {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>>;

    fn inspect_image<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImageInspection, RuntimeCapabilityError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_reference_preserves_registry_ports_and_untagged_references() {
        for (reference, name, tag) in [
            (
                "localhost:5000/team/app:v1",
                "localhost:5000/team/app",
                "v1",
            ),
            ("localhost:5000/team/app", "localhost:5000/team/app", ""),
            ("nginx:latest", "nginx", "latest"),
        ] {
            let mut image = ImageInspection {
                repo_tags: vec![reference.into()],
                ..Default::default()
            };
            image.set_display_reference();
            assert_eq!((image.name.as_str(), image.tag.as_str()), (name, tag));
        }
    }
}
