use std::collections::BTreeMap;

use serde_json::Value;

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateStack {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub drift_policy: Option<StackDriftPolicy>,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: Option<DuplicateStackSource>,
}

#[derive(Debug, Clone)]
pub struct UpdateStack {
    pub name: Option<String>,
    pub platform_id: Option<Uuid>,
    pub description: Option<Option<String>>,
    pub stack_source: Option<StackSource>,
    pub spec: Option<Value>,
    pub drift_policy: Option<StackDriftPolicy>,
    pub row_version: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RenameStack {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ApplyStack {
    pub id: Uuid,
    pub recreate: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct RollbackStack {
    pub stack_id: Uuid,
    pub release_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StackApplyOptions {
    pub expected_version: Option<i64>,
    pub service_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportComposeProject {
    pub name: String,
    pub platform_id: Uuid,
    pub project_name: String,
    pub description: Option<String>,
    pub spec: StackSpec,
    pub tag_ids: Vec<Uuid>,
    pub import_kind: StackImportKind,
    pub preview_fingerprint: String,
    pub detected_secret_values: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct DuplicateStackSource {
    pub id: Uuid,
    pub name: String,
}
