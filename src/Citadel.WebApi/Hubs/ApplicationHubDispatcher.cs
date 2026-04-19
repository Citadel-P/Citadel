using Application.Services.Abstractions;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;
using static Hosting.Common.Constants;

namespace WebApi.Hubs;

internal class ApplicationHubDispatcher(IHubContext<ApplicationHub> hubContext) : IApplicationHubDispatcher
{
    #region Container Info
    public Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainerInfoGroup(container.Id.Length > 12 ? container.Id[..12] : container.Id))
            .SendAsync("ReceiveContainerInfo", container, cancellationToken);
    #endregion

    #region Container Logs
    public Task SendContainerLogs(string containerId, ReadOnlyMemory<byte> buffer) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainerLogGroup(containerId))
            .SendAsync("SendContainerLogs", buffer);

    public Task SendContainerLogsBatchToConnection(string connectionId, byte[] recentLogs) =>
        hubContext.Clients
            .Client(connectionId)
            .SendAsync("SendContainerLogsBatch", recentLogs);
    #endregion

    #region Containers
    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainersGroup(platformId))
            .SendAsync("ContainersStatsUpdated", ContainerStatView.Map(containers));

    public Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainersGroup(platformId))
            .SendAsync("ContainersInfoUpdated", ContainersView.Map(containers));
    #endregion

    #region Images

    public Task SendImageInfo(Guid platformId, Image image) =>
       hubContext.Clients
           .Group(WellKnownSignalRGroups.ImagesGroup(platformId))
           .SendAsync("ImageInfoUpdated", ImagesView.Map(image));

    public Task SendImagesInfo(Guid platformId, IEnumerable<Image> images) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ImagesGroup(platformId))
            .SendAsync("ImagesInfoUpdated", ImagesView.Map(images));

    #endregion

    #region Platforms
    public Task PushPlatformUpdate(Platform platform) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.PlatformsGroup)
            .SendAsync("PlatformUpdated", platform.Map());

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.PlatformsGroup)
            .SendAsync("PlatformsUpdated", PlatformsView.Map(platforms).Platforms);

    public Task PlatformDeleted(Guid platformId) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.PlatformsGroup)
            .SendAsync("PlatformsDeleted", platformId);

    public Task PushPlatformStats(Guid platformId, PlatformStatsResult platform) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.PlatformsGroup)
            .SendAsync("PlatformStatsUpdated", PlatformStatsBatchView.Map(platformId, platform));
    #endregion

    #region Docker Daemon Events
    public Task SendContainerEvent(Container container, string @event) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.DockerDaemonGroup(container.PlatformId))
            .SendAsync("ContainerEventReceived", ContainerView.Map(container), @event);

    public Task SendImageEvent(Image image, string @event) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.DockerDaemonGroup(image.PlatformId))
            .SendAsync("ImageEventReceived", ImagesView.Map(image), @event);

    public Task SendVolumeEvent(DockerVolumeResult? volume, string @event, string actorId, Guid platformId) =>
         hubContext.Clients
            .Group(WellKnownSignalRGroups.DockerDaemonGroup(platformId))
            .SendAsync("VolumeEventReceived", volume, @event, actorId);

    public Task SendNetworkEvent(DockerNetworkResult? network, string @event, string actorId, Guid platformId) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.DockerDaemonGroup(platformId))
            .SendAsync("NetworkEventReceived", network, @event, actorId);
    #endregion

    #region Deployments
    public Task SendDeploymentInfo(Deployment deployment, string action)
    {
        var map = DeploymentView.Map(deployment);
        hubContext.Clients
            .Group(WellKnownSignalRGroups.DeploymentGroup(deployment.Id))
            .SendAsync("DeploymentInfoUpdated", map, action);

        hubContext.Clients
            .Group(WellKnownSignalRGroups.DeploymentsGroup)
            .SendAsync("DeploymentInfoUpdated", map, action);
        return Task.CompletedTask;
    }

    #endregion

    #region Exec Sessions
    public Task SendExecOutput(string containerId, string sessionId, byte[] data)
    {
        return hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainerExecGroup(containerId, sessionId))
            .SendAsync("SendContainerExec", data);
    }
    #endregion

    #region Activities
    public Task SendActivityInfo(ActivityEvent activity)
    {
        return hubContext.Clients
            .Group(WellKnownSignalRGroups.ActivityGroup(activity.ResourceId ?? Guid.Empty))
            .SendAsync("ActivityEventReceived", ActivityView.Map(activity));
    }
    #endregion

    #region Alerts
    public Task SendTriggeredAlertEvent(AlertEvent alertEvent)
    {
        return hubContext.Clients
            .Group(WellKnownSignalRGroups.AlertEventsGroup)
            .SendAsync("AlertEventReceived", AlertEventView.Map(alertEvent));
    }

    public Task SendUpdatedAlertEvents(IEnumerable<AlertEvent> alertEvents)
    {
        var mapped = alertEvents.Select(AlertEventView.Map).ToList();

        return hubContext.Clients
            .Group(WellKnownSignalRGroups.AlertEventsGroup)
            .SendAsync("AlertEventsUpdated", mapped);
    }

    public Task SendUnresolvedAlertCount(int count)
    {
        return hubContext.Clients
            .Group(WellKnownSignalRGroups.AlertEventsGroup)
            .SendAsync("UnresolvedAlertCount", UnresolvedAlertsCountView.Map(count));
    }
    #endregion

    #region GitRepo
    public Task SendGitRepoInfo(GitRepository repository, string action = "update")
    {
        var map = GitRepositoryView.Map(repository);
        hubContext.Clients
           .Group(WellKnownSignalRGroups.GitRepoGroup(repository.Id))
           .SendAsync("GitRepositoryInfoUpdated", map, action);

        hubContext.Clients
            .Group(WellKnownSignalRGroups.GitRepositoriesGroup)
            .SendAsync("GitRepositoryInfoUpdated", map, action);

        return Task.CompletedTask;
    }
    #endregion
}
