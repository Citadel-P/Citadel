using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using static Hosting.Common.Constants;

namespace Application.Services.Abstractions;

public interface IApplicationHubDispatcher
{
    #region Daemon events
    Task SendContainerEvent(Container container, string @event);
    Task SendImageEvent(Image image, string @event);
    #endregion

    #region Container Info
    Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken);
    #endregion

    #region Container Logs
    Task SendContainerLog(string containerId, ReadOnlyMemory<byte> buffer);
    Task SendContainerLogsBatchToConnection(string connectionId, byte[] recentLogs);
    #endregion

    #region Containers
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
    #endregion

    #region Images
    Task SendImagesInfo(Guid platformId, IEnumerable<Image> images);
    #endregion

    #region Platforms
    Task PushPlatformUpdate(Platform platform);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
    #endregion
}
