using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;

namespace Application.Services.Abstractions;

public interface IApplicationHubDispatcher
{
    Task SendLicenseStateChanged(CancellationToken cancellationToken = default);

    #region Daemon events
    Task SendContainerEvent(Container container, string @event);
    Task SendImageEvent(Image image, string @event);
    Task SendVolumeEvent(DockerVolumeResult? volume, string @event, string actorId, Guid platformId);
    Task SendNetworkEvent(DockerNetworkResult? network, string @event, string actorId, Guid platformId);
    #endregion

    #region Container Info
    Task SendContainerInfo(string containerReference, DockerContainer container, CancellationToken cancellationToken);
    #endregion

    #region Container Logs
    Task SendContainerLogs(string containerId, ReadOnlyMemory<byte> buffer);
    Task SendContainerLogsBatchToConnection(string connectionId, byte[] recentLogs);
    Task SendStackLogs(Guid stackId, ReadOnlyMemory<byte> buffer);
    Task SendStackLogsBatchToConnection(string connectionId, byte[] recentLogs);
    #endregion

    #region Containers
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
    #endregion

    #region Images
    Task SendImageInfo(Guid platformId, Image image);
    Task SendImagesInfo(Guid platformId, IEnumerable<Image> images);
    #endregion

    #region Platforms
    Task PushPlatformUpdate(Platform platform);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
    #endregion

    #region Swarm
    Task SendSwarmInventory(
        Guid platformId,
        SwarmProjectionSnapshot snapshot,
        CancellationToken cancellationToken = default);
    Task SendSwarmNodeLocalResources(
        SwarmNodeLocalResourceSnapshot snapshot,
        CancellationToken cancellationToken = default);
    Task SendSwarmNodeAgentCoverageChanged(
        Guid platformId,
        CancellationToken cancellationToken = default);
    #endregion

    #region Managed Swarm Services
    Task SendSwarmServiceInfo(SwarmService service, string action);
    #endregion

    #region Deployments
    Task SendDeploymentInfo(Deployment deployment, string action);
    #endregion

    #region Backups
    Task SendBackupRepositoryInfo(BackupRepository repository, string action);
    Task SendBackupPolicyInfo(BackupPolicy policy, string action, BackupRun? latestRun = null);
    Task SendBackupRunInfo(BackupRun run, string action);
    Task SendBackupRestoreRunInfo(BackupRestoreRun run, Guid backupPolicyId, string action);
    #endregion

    #region Builds
    Task SendBuildProjectInfo(BuildProject project, string action, BuildRun? latestRun = null);
    Task SendBuildAgentPoolInfo(BuildAgentPool pool, string action);
    Task SendBuildRunInfo(BuildRun run, string action);
    Task SendBuildRunLogs(Guid runId, IReadOnlyList<BuildRunLogEntry> entries);
    #endregion

    #region Automation
    Task SendAutomationActionInfo(AutomationAction action, string actionName);
    #endregion

    #region Stacks
    Task SendStackInfo(Stack stack, string action = "update");
    Task SendStackContainersInfo(Guid stackId, IEnumerable<DockerContainer> containers, CancellationToken cancellationToken);
    #endregion

    #region Activities
    Task SendActivityInfo(ActivityEvent activity);
    #endregion

    #region Alerts
    Task SendTriggeredAlertEvent(AlertEvent alertEvent, IEnumerable<Guid> userIds);
    Task SendUpdatedAlertEvents(IReadOnlyDictionary<Guid, IReadOnlyCollection<AlertEvent>> alertEventsByUser);
    Task SendUnresolvedAlertCounts(IReadOnlyDictionary<Guid, int> countsByUser);
    #endregion

    #region Exec Sessions
    Task SendExecOutput(string containerId, string sessionId, byte[] data);
    Task SendSwarmTaskExecOutput(Guid platformId, string taskId, string sessionId, byte[] data);
    #endregion

    #region GitRepo
    Task SendGitRepoInfo(GitRepository repository, string action = "update");
    #endregion
}
