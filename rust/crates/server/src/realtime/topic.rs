use std::fmt;
use uuid::Uuid;

/// Validated topic shape. Authorization remains the feature reader's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    Platforms,
    Deployments,
    Stacks,
    GitRepositories,
    AutomationActions,
    BackupRepositories,
    BackupPolicies,
    BuildProjects,
    BuildAgentPools,
    AlertEvents,
    Containers(Uuid),
    Images(Uuid),
    DockerDaemon(Uuid),
    Deployment(Uuid),
    Stack(Uuid),
    StackInfo(Uuid),
    SwarmService(Uuid),
    SwarmServices(Uuid),
    GitRepo(Uuid),
    AutomationAction(Uuid),
    BackupPolicy(Uuid),
    BackupRepository(Uuid),
    BackupRuns(Uuid),
    BackupRun(Uuid),
    BackupRestoreRuns(Uuid),
    BackupRestoreRun(Uuid),
    BuildProject(Uuid),
    BuildAgentPool(Uuid),
    BuildRuns(Uuid),
    BuildRun(Uuid),
    StackLog(Uuid),
    ContainerInfo(String),
    ContainerLog(String),
    ContainerExec {
        reference: String,
        session: String,
    },
    SwarmTaskExec {
        platform: Uuid,
        task: String,
        session: String,
    },
    Activity {
        kind: String,
        id: Uuid,
    },
}

impl Topic {
    pub fn parse(name: &str) -> Option<Self> {
        if name.is_empty()
            || name.len() > 256
            || name.trim() != name
            || name.chars().any(char::is_control)
        {
            return None;
        }
        let parts: Vec<_> = name.split(':').collect();
        Some(match parts.as_slice() {
            ["platforms"] => Self::Platforms,
            ["deployments"] => Self::Deployments,
            ["stacks"] => Self::Stacks,
            ["git-repositories"] => Self::GitRepositories,
            ["automation-actions"] => Self::AutomationActions,
            ["backup-repositories"] => Self::BackupRepositories,
            ["backup-policies"] => Self::BackupPolicies,
            ["build-projects"] => Self::BuildProjects,
            ["build-agent-pools"] => Self::BuildAgentPools,
            ["alert-events"] => Self::AlertEvents,
            ["containers", id] => Self::Containers(non_nil(id)?),
            ["images", id] => Self::Images(non_nil(id)?),
            ["docker-daemon", id] => Self::DockerDaemon(non_nil(id)?),
            ["deployment", id] => Self::Deployment(non_nil(id)?),
            ["stack", id] => Self::Stack(non_nil(id)?),
            ["stack-info", id] => Self::StackInfo(non_nil(id)?),
            ["swarm-service", id] => Self::SwarmService(non_nil(id)?),
            ["swarm-services", id] => Self::SwarmServices(non_nil(id)?),
            ["git-repo", id] => Self::GitRepo(non_nil(id)?),
            ["automation-action", id] => Self::AutomationAction(non_nil(id)?),
            ["backup-policy", id] => Self::BackupPolicy(non_nil(id)?),
            ["backup-repository", id] => Self::BackupRepository(non_nil(id)?),
            ["backup-runs", id] => Self::BackupRuns(non_nil(id)?),
            ["backup-run", id] => Self::BackupRun(non_nil(id)?),
            ["backup-restore-runs", id] => Self::BackupRestoreRuns(non_nil(id)?),
            ["backup-restore-run", id] => Self::BackupRestoreRun(non_nil(id)?),
            ["build-project", id] => Self::BuildProject(non_nil(id)?),
            ["build-agent-pool", id] => Self::BuildAgentPool(non_nil(id)?),
            ["build-runs", id] => Self::BuildRuns(non_nil(id)?),
            ["build-run", id] => Self::BuildRun(non_nil(id)?),
            ["stack-log", id] => Self::StackLog(non_nil(id)?),
            ["container-info", reference] if !reference.is_empty() && reference.len() <= 128 => {
                Self::ContainerInfo((*reference).into())
            }
            ["container-log", reference] if valid_container(reference) => {
                Self::ContainerLog((*reference).into())
            }
            ["container-exec", reference, session]
                if valid_container(reference) && valid_session(session) =>
            {
                Self::ContainerExec {
                    reference: (*reference).into(),
                    session: (*session).into(),
                }
            }
            ["swarm-task-exec", platform, task, session]
                if valid_session(session) && !task.is_empty() && task.len() <= 128 =>
            {
                Self::SwarmTaskExec {
                    platform: non_nil(platform)?,
                    task: (*task).into(),
                    session: (*session).into(),
                }
            }
            ["activity", kind, id] => Self::Activity {
                kind: (*kind).into(),
                id: non_nil(id)?,
            },
            _ => return None,
        })
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Platforms => "platforms",
            Self::Deployments => "deployments",
            Self::Stacks => "stacks",
            Self::GitRepositories => "git-repositories",
            Self::AutomationActions => "automation-actions",
            Self::BackupRepositories => "backup-repositories",
            Self::BackupPolicies => "backup-policies",
            Self::BuildProjects => "build-projects",
            Self::BuildAgentPools => "build-agent-pools",
            Self::AlertEvents => "alert-events",
            Self::Containers(_) => "containers",
            Self::Images(_) => "images",
            Self::DockerDaemon(_) => "docker-daemon",
            Self::Deployment(_) => "deployment",
            Self::Stack(_) => "stack",
            Self::StackInfo(_) => "stack-info",
            Self::SwarmService(_) => "swarm-service",
            Self::SwarmServices(_) => "swarm-services",
            Self::GitRepo(_) => "git-repo",
            Self::AutomationAction(_) => "automation-action",
            Self::BackupPolicy(_) => "backup-policy",
            Self::BackupRepository(_) => "backup-repository",
            Self::BackupRuns(_) => "backup-runs",
            Self::BackupRun(_) => "backup-run",
            Self::BackupRestoreRuns(_) => "backup-restore-runs",
            Self::BackupRestoreRun(_) => "backup-restore-run",
            Self::BuildProject(_) => "build-project",
            Self::BuildAgentPool(_) => "build-agent-pool",
            Self::BuildRuns(_) => "build-runs",
            Self::BuildRun(_) => "build-run",
            Self::StackLog(_) => "stack-log",
            Self::ContainerInfo(_) => "container-info",
            Self::ContainerLog(_) => "container-log",
            Self::ContainerExec { .. } => "container-exec",
            Self::SwarmTaskExec { .. } => "swarm-task-exec",
            Self::Activity { .. } => "activity",
        }
    }
    pub fn id(&self) -> Option<Uuid> {
        match self {
            Self::Containers(id)
            | Self::Images(id)
            | Self::DockerDaemon(id)
            | Self::Deployment(id)
            | Self::Stack(id)
            | Self::StackInfo(id)
            | Self::SwarmService(id)
            | Self::SwarmServices(id)
            | Self::GitRepo(id)
            | Self::AutomationAction(id)
            | Self::BackupPolicy(id)
            | Self::BackupRepository(id)
            | Self::BackupRuns(id)
            | Self::BackupRun(id)
            | Self::BackupRestoreRuns(id)
            | Self::BackupRestoreRun(id)
            | Self::BuildProject(id)
            | Self::BuildAgentPool(id)
            | Self::BuildRuns(id)
            | Self::BuildRun(id)
            | Self::StackLog(id) => Some(*id),
            Self::Activity { id, .. } => Some(*id),
            Self::SwarmTaskExec { platform, .. } => Some(*platform),
            _ => None,
        }
    }
    pub fn reference(&self) -> Option<&str> {
        match self {
            Self::ContainerInfo(reference)
            | Self::ContainerLog(reference)
            | Self::ContainerExec { reference, .. } => Some(reference),
            Self::SwarmTaskExec { task, .. } => Some(task),
            Self::Activity { kind, .. } => Some(kind),
            _ => None,
        }
    }
}
impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.kind();
        match self {
            Self::ContainerExec { reference, session } => write!(f, "{kind}:{reference}:{session}"),
            Self::SwarmTaskExec {
                platform,
                task,
                session,
            } => write!(f, "{kind}:{platform}:{task}:{session}"),
            Self::Activity { kind: resource, id } => write!(f, "{kind}:{resource}:{id}"),
            _ => {
                if let Some(id) = self.id() {
                    write!(f, "{kind}:{id}")
                } else if let Some(reference) = self.reference() {
                    write!(f, "{kind}:{reference}")
                } else {
                    f.write_str(kind)
                }
            }
        }
    }
}
fn non_nil(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok().filter(|id| !id.is_nil())
}
fn valid_container(value: &str) -> bool {
    non_nil(value).is_some()
        || (matches!(value.len(), 12 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn valid_session(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_wire_topics_round_trip_and_keep_session_strings() {
        let id = Uuid::now_v7();
        for key in [
            "platforms",
            "deployments",
            "stacks",
            "git-repositories",
            "automation-actions",
            "backup-repositories",
            "backup-policies",
            "build-projects",
            "build-agent-pools",
            "alert-events",
        ] {
            assert_eq!(Topic::parse(key).unwrap().to_string(), key);
        }
        for key in [
            "containers",
            "images",
            "docker-daemon",
            "deployment",
            "stack",
            "stack-info",
            "swarm-service",
            "swarm-services",
            "git-repo",
            "automation-action",
            "backup-policy",
            "backup-repository",
            "backup-runs",
            "backup-run",
            "backup-restore-runs",
            "backup-restore-run",
            "build-project",
            "build-agent-pool",
            "build-runs",
            "build-run",
            "stack-log",
        ] {
            let name = format!("{key}:{id}");
            assert_eq!(Topic::parse(&name).unwrap().to_string(), name);
            assert!(Topic::parse(&format!("{key}:{}", Uuid::nil())).is_none());
        }
        for name in [
            format!("container-info:{id}"),
            "container-info:arbitrary-ref".into(),
            "container-log:ABCDEF012345".into(),
            format!("container-exec:{id}:not_a-uuid"),
            format!("swarm-task-exec:{id}:task:session_1"),
            format!("activity:Deployment:{id}"),
        ] {
            assert_eq!(Topic::parse(&name).unwrap().to_string(), name);
        }
    }
    #[test]
    fn group_identity_preserves_uuid_spelling_and_rejects_malformed_topics() {
        let name = "containers:019D0000000170008001000000000024";
        let group = crate::realtime_groups::Group::parse(name).unwrap();
        assert_eq!(group.name, name);
        assert!(matches!(group.topic(), Topic::Containers(_)));
        for bad in [
            " containers:abc",
            "platforms:extra",
            "container-log:abc",
            "container-exec:ABCDEF012345:has space",
            "swarm-task-exec:bad:task:session",
            "platforms\n",
        ] {
            assert!(Topic::parse(bad).is_none(), "{bad:?}");
        }
    }
}
