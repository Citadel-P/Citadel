using Application.Services.Abstractions;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;
using static Hosting.Common.Constants;

namespace WebApi.Hubs;

internal class DockerHubDispatcher(IHubContext<DockerHub, ITypedDockerHub> hubContext) : IDockerHubDispatcher
{
    #region Container Info
    public Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken)
        => hubContext.Clients.Group(WellKnownSignalRGroups.ContainerInfoGroup(container.ContainerId)).ReceiveContainerInfo(container);
    #endregion

    #region Container Logs
    public Task SendContainerLog(string containerId, string logLine)
        => hubContext.Clients.Group(WellKnownSignalRGroups.ContainerLogGroup(containerId)).SendContainerLog(logLine);

    public Task SendContainerLogsBatchToConnection(string connectionId, IEnumerable<string> recentLogs)
        => hubContext.Clients.Client(connectionId).SendContainerLogsBatch(recentLogs);

    #endregion

    #region Containers
    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers)
        => hubContext.Clients.Group(WellKnownSignalRGroups.ContainersGroup(platformId)).ContainersStatsUpdated(ContainerStatView.Map(containers));

    public Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers)
        => hubContext.Clients.Group(WellKnownSignalRGroups.ContainersGroup(platformId)).ContainersInfoUpdated(ContainersView.Map(containers));
    #endregion

    #region Platforms
    public Task PushPlatformUpdate(Platform platform)
        => hubContext.Clients.Group(WellKnownSignalRGroups.PlatformsGroup).PlatformUpdated(platform.Map());

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms)
        => hubContext.Clients.Group(WellKnownSignalRGroups.PlatformsGroup).PlatformsUpdated(PlatformsView.Map(platforms).Platforms);

    public Task PlatformDeleted(Guid platformId)
        => hubContext.Clients.Group(WellKnownSignalRGroups.PlatformsGroup).PlatformsDeleted(platformId);

    public Task PushPlatformStats(Guid platformId, PlatformStatsResult platform)
        => hubContext.Clients.Group(WellKnownSignalRGroups.PlatformsGroup).PlatformStatsUpdated(PlatformStatsBatchView.Map(platformId, platform));

    #endregion

    #region Docker Daemon Events
    public Task SendContainerEvent(Container container, string @event)
        => hubContext.Clients.Group(WellKnownSignalRGroups.DockerDaemonGroup(container.PlatformId)).ContainerEventReceived(ContainerView.Map(container), @event);
    #endregion
}
