using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Identity;
using Hosting.Common.Models;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork : IAsyncDisposable
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IActorRepository Actors { get; }
    IImageRepository Images { get; }
    IPlatformRepository Platforms { get; }
    IRegistryRepository Registries { get; }
    IContainerRepository Containers { get; }
    IAlertRuleRepository AlertRules { get; }
    IAlertEventRepository AlertEvents { get; }
    IDeploymentRepository Deployments { get; }
    IRefreshTokenRepository RefreshTokens { get; }
    IPlatformStatRepository PlatformStats { get; }
    IContainerStatRepository ContainerStats { get; }
    IActivityEventRepository ActivityEventRepository { get; }

    Task CommitAsync(CancellationToken cancellationToken);
    Task RollbackAsync();
}

public interface IActorRepository
{
    Task<Actor?> GetById(Guid id, CancellationToken cancellationToken);
}

public interface IUserRepository 
{
    Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken);
}

public interface IRegistryRepository 
{
    Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Registry registry, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IContainerRepository 
{
    Task<Container?> GetByIdAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<Container?> GetContainerInfoAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetStuckContainersAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<Container?> GetByDeploymentIdAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByDeploymentIdsAsync(IEnumerable<Guid> deploymentIds, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>?> GetContainersInfoAsync(Guid platformId, CancellationToken cancellationToken);

    Task<int> AddAsync(Container container, CancellationToken cancellationToken);
    Task<int> BulkUpsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken);

    Task<int> UpdateAsync(Container container, CancellationToken cancellationToken);
    Task<int> UpdateContainersStateAsync(IEnumerable<Guid> ids, ContainerStateStatus state, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);

    Task<int> DeleteAsync(IEnumerable<Guid> containersId, CancellationToken cancellationToken);
}

public interface IContainerStatRepository 
{
    Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string containerId, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken);
    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface IActivityEventRepository 
{
    Task<int> AddAsync(ActivityEvent activityEvent, CancellationToken cancellationToken);
    Task<ActivityEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<PagedResult<ActivityEvent>> GetPagedAsync(Guid? resourceId, ActivityResourceType? resourceType, ActivityEventType? eventType, int page,
        int pageSize, CancellationToken cancellationToken);

    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface IRefreshTokenRepository 
{
    Task<int> CountAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> AddAsync(RefreshToken refreshToken, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByRefreshTokenIdAsync(Guid id, CancellationToken cancellationToken);

    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> DeleteOldestTokensAsync(Guid userId, int tokensToRemoveCount, CancellationToken cancellationToken);
}

public interface IPlatformStatRepository 
{
    Task<IEnumerable<PlatformStat>> GetStatsAggregatedLast24HoursAsync(Guid platformId, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<PlatformStat> stats, CancellationToken cancellationToken);
    Task<int> RemoveOlderThanAsync(long createdBeforeEpochSeconds, CancellationToken cancellationToken);
}

public interface ITeamRepository { }

public interface IPlatformRepository 
{
    Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken);
    Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByContainerIdAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetInfoAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken);

    Task<int?> PlatformNameExistsAsync(string name, Guid excludePlatformId, CancellationToken cancellationToken);
    Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken);

    Task<int> AddAsync(Platform platform, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Platform platform, CancellationToken cancellationToken);

    Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken);
}

public interface IImageRepository
{
    Task<Image?> GetByIdAsync(Guid id, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetByIdAsync(string[] ids, Guid platformId, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetStuckImagesAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<Image?> GetByDockerImageIdAsync(string dockerImageId, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Image>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);

    Task<int> AddOrUpdateAsync(Image image, CancellationToken cancellationToken);
    Task<int> BulkUpsertAsync(IEnumerable<Image> images, CancellationToken cancellationToken);

    Task<int> DeleteAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IDeploymentRepository
{
    Task<Deployment?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Deployment?> GetInfoAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetInfoAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetStuckDeploymentsAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<bool> ExistsAsync(string name, Guid platformId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, Guid platformId, CancellationToken cancellationToken);
    Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Deployment deployment, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, DeploymentStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
    Task<int> UpdateStatusAsync(IEnumerable<Guid> ids, DeploymentStatus status, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IAlertRuleRepository
{
    Task<AlertRule?> GetByIdAsync(Guid alertRuleId, CancellationToken cancellationToken);
    Task<AlertRuleState?> GetStateAsync(Guid alertRuleId, Guid resourceId, CancellationToken cancellationToken);
    Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> AddAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> UpdateStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken);
}

public interface IAlertEventRepository
{
    Task<int> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<PagedResult<AlertEvent>> GetPagedAsync(Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken);
}