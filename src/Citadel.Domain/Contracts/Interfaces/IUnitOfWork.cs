using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Models;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork : IAsyncDisposable
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IRoleRepository Roles { get; }
    IActorRepository Actors { get; }
    IImageRepository Images { get; }
    IPlatformRepository Platforms { get; }
    IRegistryRepository Registries { get; }
    IGitAccountRepository GitAccounts { get; }
    IGitReposRepository GitRepositories { get; }
    IContainerRepository Containers { get; }
    IAlertRuleRepository AlertRules { get; }
    IAlertEventRepository AlertEvents { get; }
    IDeploymentRepository Deployments { get; }
    IStackRepository Stacks { get; }
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
    Task<int> AddAsync(Actor actor, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Actor actor, CancellationToken cancellationToken);
}

public interface IUserRepository 
{
    Task<bool> HasPermissionAsync(Guid userId, ResourceType resourceType, ResourceAction action, Guid? resourceId, CancellationToken ct);
    Task<UserAuthInfo?> GetUserAuthInfoByEmailOrNameAsync(string emailOrName, CancellationToken cancellationToken);
    Task<User?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<User>> GetAllAsync(CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, ResourceAction action, int page, int pageSize, CancellationToken cancellationToken);
    Task<IEnumerable<User>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<(bool NameExists, bool EmailExists)> GetConflictsAsync(string name, string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<(User? User, bool IsEnabled, bool NameExists, bool EmailExists)> GetUserUpdateStateAsync(Guid id, string? name, string? email, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<bool> ExistsByEmailAsync(string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(User user, CancellationToken cancellationToken);
    Task<int> UpdateAsync(User user, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetTeamIdsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceTeamsAsync(Guid userId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken);
}

public interface IRegistryRepository 
{
    Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Registry registry, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IStackRepository
{
    Task<Stack?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetInfoAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<StackRelease>> GetReleasesByStackIdAsync(Guid stackId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Stack stack, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Stack stack, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IGitReposRepository
{
    Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetWithAccountAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(GitRepository gitRepository, CancellationToken cancellationToken);
    Task<int> UpdateAsync(GitRepository gitRepository, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IGitAccountRepository
{
    Task<GitAccount?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitAccount?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);
    Task<bool> IsNameTakenAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(GitAccount gitAccount, CancellationToken cancellationToken);
    Task<int> UpdateAsync(GitAccount gitAccount, CancellationToken cancellationToken);
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

public interface ITeamRepository
{
    Task<Team?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<TeamDetails?> GetDetailsAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Team>> GetAllAsync(CancellationToken cancellationToken);
    Task<PagedResult<TeamDetails>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken);
    Task<PagedResult<TeamDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, ResourceAction action, int page, int pageSize, CancellationToken cancellationToken);
    Task<IEnumerable<Team>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> GetConflictsAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<(Team? Team, bool IsEnabled, bool NameExists)> GetTeamUpdateStateAsync(Guid id, string? name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(Team team, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Team team, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetUserIdsAsync(Guid teamId, CancellationToken cancellationToken);
    Task<(TeamDetails? Team, bool UserExists, bool HasMember)> GetMemberAssignmentStateAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> AddMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> RemoveMemberAsync(Guid teamId, Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceMembersAsync(Guid teamId, IEnumerable<Guid> userIds, CancellationToken cancellationToken);
}

public interface IRoleRepository
{
    Task<Role?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<IEnumerable<Role>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(Role role, CancellationToken cancellationToken);
    Task<int> RenameAsync(Role role, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken);
    Task<int> AddActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> RemoveActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplaceActorRolesAsync(Guid actorId, IEnumerable<Guid> roleIds, CancellationToken cancellationToken);
    Task<IEnumerable<Permission>> GetPermissionsAsync(Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplacePermissionsAsync(Guid roleId, IEnumerable<Permission> permissions, CancellationToken cancellationToken);
}

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
    Task<IEnumerable<Deployment>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
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
    Task<IEnumerable<AlertRule>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<AlertRuleState?> GetStateAsync(Guid alertRuleId, Guid resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<PagedResult<AlertRule>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken);
    Task<AlertChannel?> GetChannelByIdAsync(Guid channelId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAllChannelsAsync(CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAuthorizedChannelsAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken);
    Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> AddChannelAsync(AlertChannel alertChannel, CancellationToken cancellationToken);
    Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> UpsertAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken);
    Task<int> UpdateChannelAsync(AlertChannel channel, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<int> RemoveChannelsRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IAlertEventRepository
{
    Task<int> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<AlertEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<AlertEvent>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<PagedResult<AlertEvent>> GetAuthorizedPagedAsync(Guid userId, ResourceType permissionResourceType, ResourceAction action, Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<PagedResult<AlertEvent>> GetPagedAsync(Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<int> UpdateAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<int> BulkUpdateAsync(IEnumerable<AlertEvent> alertEvents, CancellationToken cancellationToken);
    Task<int> CountUnresolvedAsync(CancellationToken cancellationToken);
}