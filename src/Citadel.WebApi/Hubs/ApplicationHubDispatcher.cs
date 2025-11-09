using Application.Services.Abstractions;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
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
    public Task SendContainerLog(string containerId, ReadOnlyMemory<byte> buffer) =>
        hubContext.Clients
            .Group(WellKnownSignalRGroups.ContainerLogGroup(containerId))
            .SendAsync("SendContainerLog", buffer);

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
    #endregion
}
