use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeNetwork {
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: Option<bool>,
    pub attachable: Option<bool>,
    pub ingress: Option<bool>,
    #[serde(rename = "enableIPv6", alias = "enableIpv6")]
    pub enable_ipv6: Option<bool>,
    #[serde(rename = "enableIPv4", alias = "enableIpv4")]
    pub enable_ipv4: Option<bool>,
    pub config_only: Option<bool>,
    pub ipam: Option<RuntimeIpam>,
    pub config_from: Option<RuntimeConfigFrom>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIpam {
    pub driver: String,
    #[serde(default)]
    pub config: Vec<RuntimeIpamConfig>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIpamConfig {
    pub subnet: Option<String>,
    pub ip_range: Option<String>,
    pub gateway: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigFrom {
    pub network: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedRuntimeNetwork {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeVolume {
    pub name: String,
    pub driver: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

/// Composite for orchestration only; scoped services use individual capabilities.
pub trait PlatformResourceMutationPort:
    crate::NetworkMutationPort + crate::VolumeMutationPort
{
}
impl<T: crate::NetworkMutationPort + crate::VolumeMutationPort + ?Sized>
    PlatformResourceMutationPort for T
{
}
