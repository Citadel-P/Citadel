use super::*;
impl StackService {
    pub async fn preflight_swarm(
        &self,
        compose_files: &[String],
        bindings: &[crate::StackBuildImageBinding],
    ) -> Result<crate::SwarmStackCompatibilityReport, StackError> {
        analyze_swarm_compatibility(compose_files, bindings)
    }

    pub async fn import_draft(
        &self,
        platform_id: Uuid,
        project_name: &str,
        import_kind: Option<crate::StackImportKind>,
    ) -> Result<ComposeProjectImportDraft, StackError> {
        let claim = self
            .runtime
            .import_claim(
                platform_id,
                project_name,
                import_kind,
                &self.shutdown.child_token(),
            )
            .await?;
        let existing = self
            .store
            .find_import_owner(platform_id, project_name)
            .await?;
        let mut issues = Vec::new();
        if existing.is_some() {
            issues.push(StackAdoptionIssue {
                code: "ExistingCitadelOwnership".to_owned(),
                message: "This runtime project is already owned by a Citadel Stack.".to_owned(),
                severity: "Error".to_owned(),
                field_path: None,
            });
        }
        let runtime_fingerprint = import_runtime_fingerprint(&claim);
        Ok(ComposeProjectImportDraft {
            import_kind: claim.import_kind,
            source: ComposeProjectImportSource {
                platform_id,
                platform_name: claim.platform_name,
                project_name: project_name.to_owned(),
                container_ids: claim.container_ids,
                container_names: claim.container_names,
                services: claim.services,
            },
            draft: ComposeProjectStackDraft {
                name: project_name.to_owned(),
                platform_id,
                description: Some(format!("Imported from Docker project '{project_name}'.")),
                drift_policy: crate::StackDriftPolicy::default(),
                tag_ids: Vec::new(),
            },
            issues,
            runtime_fingerprint,
        })
    }

    pub async fn validate_import(
        &self,
        platform_id: Uuid,
        project_name: &str,
        name: &str,
        stack_source: crate::StackSource,
        spec: &StackSpec,
        import_kind: Option<crate::StackImportKind>,
    ) -> Result<ComposeProjectImportValidation, StackError> {
        let mut name = name.to_owned();
        normalize_name(&mut name)?;
        if spec.source() != stack_source {
            return Err(validation(
                "Stack source and specification type must match.",
            ));
        }
        spec.validate()?;
        let claim = self
            .runtime
            .import_claim(
                platform_id,
                project_name,
                import_kind,
                &self.shutdown.child_token(),
            )
            .await?;
        if import_kind.is_some_and(|kind| kind != claim.import_kind) {
            return Err(validation(
                "The requested import kind does not match the Platform runtime.",
            ));
        }
        let source = self
            .materialize_import_source(platform_id, project_name, &name, spec, claim.import_kind)
            .await?;
        let compose_files = source.compose_contents()?;
        let desired = parse_compose(&compose_files)?;
        let desired_by_name = desired
            .services
            .iter()
            .map(|service| (service.name.as_str(), service.image.clone()))
            .collect::<BTreeMap<_, _>>();
        let runtime_by_name = claim
            .services
            .iter()
            .map(|service| (service.name.as_str(), service))
            .collect::<BTreeMap<_, _>>();
        let names = desired_by_name
            .keys()
            .chain(runtime_by_name.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        let services: Vec<ComposeProjectServiceComparison> = names
            .iter()
            .map(|name| {
                let runtime = runtime_by_name.get(name);
                ComposeProjectServiceComparison {
                    name: (*name).to_owned(),
                    runtime_container_count: runtime.map_or(0, |value| value.container_count),
                    runtime_image: runtime.and_then(|value| value.image.clone()),
                    defined_in_source: desired_by_name.contains_key(name),
                    source_image: desired_by_name.get(name).cloned().flatten(),
                }
            })
            .collect();
        let mut issues = Vec::new();
        if claim.import_kind == crate::StackImportKind::SwarmStack {
            let compatibility =
                analyze_swarm_compatibility(&compose_files, &spec.common().build_image_bindings)?;
            issues.extend(compatibility.issues.into_iter().map(|issue| {
                StackAdoptionIssue {
                    code: issue.code,
                    message: issue.message,
                    severity: match issue.severity {
                        crate::SwarmStackCompatibilitySeverity::Warning => "Warning",
                        crate::SwarmStackCompatibilitySeverity::Error => "Error",
                    }
                    .to_owned(),
                    field_path: issue.field_path,
                }
            }));
        }
        for comparison in &services {
            if !comparison.defined_in_source {
                issues.push(StackAdoptionIssue {
                    code: "MissingSourceService".to_owned(),
                    message: format!(
                        "Runtime Service '{}' is not defined by the selected source.",
                        comparison.name
                    ),
                    severity: "Error".to_owned(),
                    field_path: Some(format!("services.{}", comparison.name)),
                });
            }
        }
        let spec_json =
            serde_json::to_string(spec).map_err(|error| StackError::Storage(error.to_string()))?;
        let preview_fingerprint = compose_digest(&[
            import_runtime_fingerprint(&claim),
            spec_json,
            source.resolved_commit_sha.unwrap_or_default(),
        ]);
        Ok(ComposeProjectImportValidation {
            services,
            issues,
            preview_fingerprint,
            importable_sensitive_environment_names: Vec::new(),
            can_import_sensitive_environment_values: false,
        })
    }

    pub async fn import(
        &self,
        actor: ActorId,
        administrator: bool,
        mut input: ImportComposeProject,
    ) -> Result<StackDetails, StackError> {
        normalize_name(&mut input.name)?;
        normalize_description(&mut input.description)?;
        input.spec.validate()?;
        let claim = self
            .runtime
            .import_claim(
                input.platform_id,
                &input.project_name,
                Some(input.import_kind),
                &self.shutdown.child_token(),
            )
            .await?;
        let source = self
            .materialize_import_source(
                input.platform_id,
                &input.project_name,
                &input.name,
                &input.spec,
                claim.import_kind,
            )
            .await?;
        let compose_files = source.compose_contents()?;
        let desired = parse_compose(&compose_files)?;
        if claim.import_kind == crate::StackImportKind::SwarmStack
            && let Some(issue) = analyze_swarm_compatibility(
                &compose_files,
                &input.spec.common().build_image_bindings,
            )?
            .issues
            .into_iter()
            .find(|issue| issue.severity == crate::SwarmStackCompatibilitySeverity::Error)
        {
            return Err(validation(&issue.message));
        }
        let desired_names = desired
            .services
            .iter()
            .map(|service| service.name.as_str())
            .collect::<BTreeSet<_>>();
        if let Some(service) = claim
            .services
            .iter()
            .find(|service| !desired_names.contains(service.name.as_str()))
        {
            return Err(validation(&format!(
                "Runtime Service '{}' is not defined by the selected source.",
                service.name
            )));
        }
        let spec_json = serde_json::to_string(&input.spec)
            .map_err(|error| StackError::Storage(error.to_string()))?;
        let expected_fingerprint = compose_digest(&[
            import_runtime_fingerprint(&claim),
            spec_json,
            source.resolved_commit_sha.unwrap_or_default(),
        ]);
        if expected_fingerprint != input.preview_fingerprint
            || claim.import_kind != input.import_kind
        {
            return Err(StackError::Conflict(
                "The runtime Stack changed after it was reviewed. Refresh the import draft."
                    .to_owned(),
            ));
        }
        let imported = self
            .store
            .import(actor, administrator, &input, &claim)
            .await?;
        self.notifier.changed(imported.id, "imported");
        Ok(imported)
    }

    pub(super) async fn materialize_import_source(
        &self,
        platform_id: Uuid,
        project_name: &str,
        name: &str,
        spec: &StackSpec,
        import_kind: crate::StackImportKind,
    ) -> Result<crate::StackApplySource, StackError> {
        match spec {
            StackSpec::WebEditor { compose_file, .. } => Ok(crate::StackApplySource {
                files: vec![StackSourceFile {
                    relative_path: "compose.yml".to_owned(),
                    content: compose_file.as_bytes().to_vec(),
                }],
                compose_paths: vec!["compose.yml".to_owned()],
                env_file_paths: Vec::new(),
                working_directory: ".".to_owned(),
                labels_override_path: None,
                resolved_commit_sha: None,
            }),
            StackSpec::Git { .. } => {
                let materializer = self.source_materializer.as_ref().ok_or_else(|| {
                    validation("Git Stack source materialization is unavailable.")
                })?;
                let cancellation = self.shutdown.child_token();
                materializer
                    .materialize(
                        &StackOperationClaim {
                            stack_id: Uuid::nil(),
                            release_id: Uuid::nil(),
                            platform_id,
                            name: name.to_owned(),
                            project_name: project_name.to_owned(),
                            platform_type: match import_kind {
                                crate::StackImportKind::ComposeProject => {
                                    citadel_platforms::PlatformKind::Docker
                                }
                                crate::StackImportKind::SwarmStack => {
                                    citadel_platforms::PlatformKind::DockerSwarm
                                }
                            },
                            spec: spec.clone(),
                            row_version: 0,
                            actor_id: Uuid::nil(),
                            operation: "ValidateImport".to_owned(),
                            service_names: Vec::new(),
                        },
                        &cancellation,
                    )
                    .await
            }
        }
    }
}
