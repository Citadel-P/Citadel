using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;

namespace Application.Services.Abstractions;

public interface IDockerHubDispatcher
{
    #region Daemon events
    Task SendContainerEvent(Container container, string @event);
    #endregion

    #region Container Info
    Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken);
    #endregion

    #region Container Logs
    Task SendContainerLog(string containerId, string logLine);
    Task SendContainerLogsBatchToConnection(string connectionId, IEnumerable<string> recentLogs);
    #endregion

    #region Containers
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
    #endregion

    #region Platforms
    Task PushPlatformUpdate(Platform platform);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
    #endregion
}
