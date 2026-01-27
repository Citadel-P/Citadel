using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;

namespace Application.Services.Abstractions;

public interface IApplicationHubDispatcher
{
    #region Daemon events
    Task SendContainerEvent(Container container, string @event);
    Task SendImageEvent(Image image, string @event);
    Task SendVolumeEvent(DockerVolumeResult? volume, string @event, string actorId, Guid platformId);
    Task SendNetworkEvent(DockerNetworkResult? network, string @event, string actorId, Guid platformId);
    #endregion

    #region Container Info
    Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken);
    #endregion

    #region Container Logs
    Task SendContainerLogs(string containerId, ReadOnlyMemory<byte> buffer);
    Task SendContainerLogsBatchToConnection(string connectionId, byte[] recentLogs);
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

    #region Deployments
    Task SendDeploymentInfo(Deployment deployment);
    #endregion

    #region Exec Sessions
    Task SendExecOutput(string containerId, byte[] data);
    #endregion
}
