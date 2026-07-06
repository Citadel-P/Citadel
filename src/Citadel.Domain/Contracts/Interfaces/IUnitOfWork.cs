using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Oidc;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Tags;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Oidc;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Models;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork : IAsyncDisposable
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IRoleRepository Roles { get; }
    IActorRepository Actors { get; }
    IResourceAccessRepository ResourceAccesses { get; }
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
    IResourceBindingRepository ResourceBindings { get; }
    ISecretDefinitionRepository SecretDefinitions { get; }
    ISecretProviderRepository SecretProviders { get; }
    ITagRepository Tags { get; }
    IResourceTagRepository ResourceTags { get; }
    IOidcProviderRepository OidcProviders { get; }
    IOidcLoginStateRepository OidcLoginStates { get; }
    IOidcExternalLoginRepository OidcExternalLogins { get; }
    IRefreshTokenRepository RefreshTokens { get; }
    IPlatformStatRepository PlatformStats { get; }
    IContainerStatRepository ContainerStats { get; }
    IActivityEventRepository ActivityEventRepository { get; }

    Task CommitAsync(CancellationToken cancellationToken);
    Task RollbackAsync();
}

public interface IOidcProviderRepository
{
    Task<int> AddAsync(OidcProvider provider, CancellationToken cancellationToken);
    Task<int> UpdateAsync(OidcProvider provider, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<OidcProvider?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<OidcProvider>> GetEnabledAsync(CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
}

public interface IOidcLoginStateRepository
{
    Task<int> AddAsync(OidcLoginState state, CancellationToken cancellationToken);
    Task<OidcLoginState?> GetByStateHashAsync(string stateHash, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken);
}

public interface IOidcExternalLoginRepository
{
    Task<OidcExternalLogin?> GetAsync(Guid providerId, string subject, CancellationToken cancellationToken);
    Task<int> AddAsync(OidcExternalLogin externalLogin, CancellationToken cancellationToken);
    Task<int> UpdateSeenAsync(Guid id, string? email, DateTime updatedAt, CancellationToken cancellationToken);
    Task<int> DeleteByProviderIdAsync(Guid providerId, CancellationToken cancellationToken);
}

public interface IResourceBindingRepository
{
    Task<int> AddAsync(ResourceBinding entry, CancellationToken cancellationToken);
    Task<int> UpdateAsync(ResourceBinding entry, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<int> ReplaceEntriesAsync(ResourceBindingScope scope, Guid? resourceId, IEnumerable<ResourceBinding> entries, CancellationToken cancellationToken);
    Task<int> ReplaceResourceEntriesAsync(ResourceBindingScope scope, Guid resourceId, IEnumerable<ResourceBinding> entries, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceBinding>> GetEntriesAsync(ResourceBindingScope scope, Guid? resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceBinding>> GetEffectiveEntriesAsync(ResourceBindingScope scope, Guid resourceId, CancellationToken cancellationToken);
}

public interface ISecretDefinitionRepository
{
    Task<int> AddAsync(SecretDefinition secret, InternalSecretValue? value, CancellationToken cancellationToken);
    Task<int> UpdateAsync(SecretDefinition secret, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<SecretDefinition?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<SecretDefinition>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<SecretDefinition>> GetBoundAsync(ResourceBindingScope scope, Guid? resourceId, CancellationToken cancellationToken);
    Task<InternalSecretValue?> GetInternalValueAsync(Guid secretId, CancellationToken cancellationToken);
    Task<int> DeleteExternalByProviderIdAsync(Guid providerId, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
    Task<bool> IsUsedByResourceBindingAsync(Guid id, CancellationToken cancellationToken);
}

public interface ISecretProviderRepository
{
    Task<int> AddAsync(SecretProvider provider, CancellationToken cancellationToken);
    Task<int> UpdateAsync(SecretProvider provider, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
    Task<SecretProvider?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<SecretProvider>> GetAllAsync(CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken);
    Task<bool> IsUsedByResourceBindingAsync(Guid id, CancellationToken cancellationToken);
}

public interface ITagRepository
{
    Task<IReadOnlyList<TagWithUsage>> ListAsync(CancellationToken cancellationToken);
    Task<Tag?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Tag?> GetByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameAsync(string normalizedName, CancellationToken cancellationToken);
    Task<bool> ExistsByNormalizedNameExceptAsync(string normalizedName, Guid id, CancellationToken cancellationToken);
    Task<int> AddAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Tag tag, CancellationToken cancellationToken);
    Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken);
}

public interface IResourceTagRepository
{
    Task<IReadOnlyList<TagSummary>> GetForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        CancellationToken cancellationToken);

    Task<IReadOnlyDictionary<Guid, IReadOnlyList<TagSummary>>> GetForResourcesAsync(
        TaggableResourceType resourceType,
        IReadOnlyCollection<Guid> resourceIds,
        CancellationToken cancellationToken);

    Task<int> ReplaceForResourceAsync(
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid> tagIds,
        Guid createdByActorId,
        DateTime now,
        CancellationToken cancellationToken);

    Task<bool> AllTagsExistAsync(
        IReadOnlyCollection<Guid> tagIds,
        CancellationToken cancellationToken);
}

public interface IExternalSecretProviderClient
{
    Task<ExternalSecretProviderConnectionTestResult> TestConnectionAsync(
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken);

    Task<ExternalSecretValueResult> ResolveAsync(
        SecretDefinition secret,
        SecretProvider provider,
        string token,
        CancellationToken cancellationToken);
}

public sealed record ExternalSecretProviderConnectionTestResult(bool Success, string Message)
{
    public static ExternalSecretProviderConnectionTestResult Succeeded(string message) => new(true, message);

    public static ExternalSecretProviderConnectionTestResult Failed(string message) => new(false, message);
}

public sealed record ExternalSecretValueResult(bool IsSuccess, string? Value, string? ErrorMessage)
{
    public static ExternalSecretValueResult Success(string value) => new(true, value, null);

    public static ExternalSecretValueResult Failure(string errorMessage) => new(false, null, errorMessage);
}

public interface IActorRepository
{
    Task<Actor?> GetById(Guid id, CancellationToken cancellationToken);
    Task<int> AddAsync(Actor actor, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Actor actor, CancellationToken cancellationToken);
}

public interface IResourceAccessRepository
{
    Task<IEnumerable<ResourceAccessDetails>> GetAllByActorIdAsync(Guid actorId, CancellationToken cancellationToken);
    Task<int> AddAsync(ResourceAccess resourceAccess, CancellationToken cancellationToken);
    Task<int> RemoveAsync(
        Guid actorId,
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission>? specificPermissions,
        CancellationToken cancellationToken);
    Task<int> ReplaceAsync(Guid actorId, IEnumerable<ResourceAccess> resourceAccesses, CancellationToken cancellationToken);
}

public interface IUserRepository 
{
    Task<PermissionMetadata> GetEffectivePermissionsAsync(
        Guid[] actorIds, 
        ResourceType resourceType, 
        Guid? resourceId, 
        CancellationToken ct);
    
    Task<IReadOnlyDictionary<Guid, PermissionMetadata>> GetEffectivePermissionsBatchAsync(
        Guid[] actorIds, 
        ResourceType resourceType, 
        Guid[] resourceIds, 
        CancellationToken ct = default);

    Task<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken ct);
    Task<UserAuthInfo?> GetUserAuthInfoByEmailOrNameAsync(string emailOrName, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken);
    Task<User?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<UserDetails?> GetDetailsAsync(Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<User>> GetAllAsync(CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<PagedResult<UserDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<IEnumerable<UserSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<UserSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, string query, int limit, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<User>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<(bool NameExists, bool EmailExists)> GetConflictsAsync(string name, string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<(User? User, bool IsEnabled, bool NameExists, bool EmailExists)> GetUserUpdateStateAsync(Guid id, string? name, string? email, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid excludeId, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<bool> ExistsByEmailAsync(string email, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(User user, CancellationToken cancellationToken);
    Task<int> UpdateAsync(User user, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetTeamsLookupAsync(Guid sourceUserId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetTeamIdsAsync(Guid userId, CancellationToken cancellationToken);
    Task<int> ReplaceTeamsAsync(Guid userId, IEnumerable<Guid> teamIds, CancellationToken cancellationToken);
}

public interface IRegistryRepository
{
    Task<Registry?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Registry?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Registry>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Registry>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Registry registry, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IStackRepository
{
    Task<Stack?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<Stack?> GetInfoAsync(Guid id, CancellationToken cancellationToken);
    Task<StackDriftStack?> GetDriftStackAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<StackDriftStack>> GetDriftMonitorStacksAsync(CancellationToken cancellationToken);
    Task<IEnumerable<string>> GetContainerIdsAsync(Guid stackId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetInfoAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Container>> GetContainersAsync(Guid stackId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> CanAccessAsync(Guid userId, Guid stackId, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetGitRepositoryLookupAsync(Guid stackId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<StackRelease>> GetReleasesByStackIdAsync(Guid stackId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Stack stack, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> AddReleaseAsync(StackRelease release, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Stack stack, CancellationToken cancellationToken);
    Task<int> UpdateReleaseStatusAsync(Guid releaseId, StackReleaseStatus status, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetStuckStacksAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<IEnumerable<GitStackBranchSubscription>> GetGitStackBranchSubscriptionsAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Stack>> GetBranchTrackingGitStacksAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken);
    Task<bool> UpdateProcessingAsync(Guid id, StackReleaseStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
}

public interface IGitReposRepository
{
    Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetWithAccountAsync(Guid id, CancellationToken cancellationToken);
    Task<GitRepository?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<GitRepository>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(GitRepository gitRepository, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(GitRepository gitRepository, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<GitRepositoryRef?> GetRefAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken);
    Task<IEnumerable<GitRepositoryRef>> GetRefsByRepositoryIdAsync(Guid gitRepositoryId, CancellationToken cancellationToken);
    Task<int> UpsertRefAsync(GitRepositoryRef gitRepositoryRef, CancellationToken cancellationToken);
}

public interface IGitAccountRepository
{
    Task<GitAccount?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<GitAccount?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<GitAccount>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
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
    Task<IEnumerable<Container>> GetByIdsAsync(string[] dockerContainerIds, CancellationToken cancellationToken);
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
    Task<IEnumerable<ContainerStat>> GetStatsAggregatedAsync(string containerId, int hours, CancellationToken cancellationToken);
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
    Task<PagedResult<TeamDetails>> GetPagedAsync(int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<PagedResult<TeamDetails>> GetAuthorizedPagedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, int page, int pageSize, string? name, CancellationToken cancellationToken);
    Task<IEnumerable<TeamSearchItem>> SearchAsync(string query, int limit, CancellationToken cancellationToken);
    Task<IEnumerable<TeamSearchItem>> SearchAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, string query, int limit, CancellationToken cancellationToken);
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
    Task<IEnumerable<Guid>> GetUserIdsByActorIdAsync(Guid actorId, CancellationToken cancellationToken);
}

public interface IRoleRepository
{
    Task<Role?> GetAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAllAsync(CancellationToken cancellationToken);
    Task<IEnumerable<Role>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<Role>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> ExistsByNameAsync(string name, Guid? excludeId, CancellationToken cancellationToken);
    Task<int> AddAsync(Role role, CancellationToken cancellationToken);
    Task<int> RenameAsync(Role role, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetUserRoleLookupAsync(Guid sourceUserId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetActorRoleIdsAsync(Guid actorId, CancellationToken cancellationToken);
    Task<int> AddActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> RemoveActorRoleAsync(Guid actorId, Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplaceActorRolesAsync(Guid actorId, IEnumerable<Guid> roleIds, CancellationToken cancellationToken);
    Task<IEnumerable<Permission>> GetPermissionsAsync(Guid roleId, CancellationToken cancellationToken);
    Task<int> ReplacePermissionsAsync(Guid roleId, IEnumerable<Permission> permissions, CancellationToken cancellationToken);
    Task<IDictionary<Guid, Guid[]>> GetActorRoleIdsAsync(IEnumerable<Guid> actorIds, CancellationToken cancellationToken);
    Task<IEnumerable<Guid>> GetActorIdsByRoleIdAsync(Guid roleId, CancellationToken cancellationToken);
}

public interface IPlatformRepository
{
    Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Platform>> GetAuthorizedWithLatestStatAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<IEnumerable<Platform>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByContainerIdAsync(string dockerContainerId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetInfoAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetDeploymentLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetStackLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid platformId, Guid userId, CancellationToken cancellationToken);

    Task<int?> PlatformNameExistsAsync(string name, Guid excludePlatformId, CancellationToken cancellationToken);
    Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken);

    Task<int> AddAsync(Platform platform, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Platform platform, CancellationToken cancellationToken);
    public Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken);

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
    Task<IEnumerable<Deployment>> GetInfoAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<string?> GetContainerIdAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformByDeploymentIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetAuthorizedInfoAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null);
    Task<bool> CanAccessAsync(Guid userId, Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetPlatformLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetRegistryLookupAsync(Guid deploymentId, Guid userId, CancellationToken cancellationToken);
    Task<IEnumerable<ResourceInfo>> GetImageLookupAsync(Guid deploymentId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<Deployment>> GetStuckDeploymentsAsync(int timeout_s = 60, CancellationToken cancellationToken = default);
    Task<bool> ExistsAsync(Guid platformId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, Guid platformId, CancellationToken cancellationToken);
    Task<bool> ExistsAsync(Guid id, string name, Guid platformId, CancellationToken cancellationToken);
    Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null);
    Task<int> UpdateAsync(Deployment deployment, CancellationToken cancellationToken);
    Task<int> UpdateProcessingAsync(Guid id, DeploymentStatus status, ResourceControlState state, long? startedAt, long rowVersion, bool? checkRowVersion, Guid? controlTriggeredBy, CancellationToken cancellationToken);
    Task<int> UpdateStatusAsync(IEnumerable<Guid> ids, DeploymentStatus status, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}

public interface IAlertRuleRepository
{
    Task<AlertRule?> GetByIdAsync(Guid alertRuleId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<AlertRuleState?> GetStateAsync(Guid alertRuleId, Guid resourceId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertRule>> GetAllAsync(CancellationToken cancellationToken = default);
    Task<PagedResult<AlertRule>> GetPagedAsync(int page, int pageSize, CancellationToken cancellationToken);
    Task<AlertChannel?> GetChannelByIdAsync(Guid channelId, CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAllChannelsAsync(CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAuthorizedChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<IEnumerable<AlertChannel>> GetAuthorizedAlertChannelsAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<int> AddAlertRuleAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> AddChannelAsync(AlertChannel alertChannel, CancellationToken cancellationToken);
    Task<int> UpdateAsync(AlertRule alertRule, CancellationToken cancellationToken);
    Task<int> UpsertAlertRuleStateAsync(AlertRuleState alertRuleState, CancellationToken cancellationToken);
    Task<int> UpdateChannelAsync(AlertChannel channel, CancellationToken cancellationToken);
    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<int> RemoveChannelsRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<bool> CanAccessAsync(Guid userId, Guid alertRuleId, CancellationToken cancellationToken);
}

public interface IAlertEventRepository
{
    Task<Guid> AddAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<AlertEvent?> GetByIdAsync(Guid id, CancellationToken cancellationToken);
    Task<IEnumerable<AlertEvent>> GetByIdAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
    Task<PagedResult<AlertEvent>> GetAuthorizedPagedAsync(Guid userId, ResourceType permissionResourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<PagedResult<AlertEvent>> GetPagedAsync(Guid? resourceId, AlertType? alertType, AlertResourceType? resourceType,
        int page, int pageSize, CancellationToken cancellationToken, bool? unresolvedOnly = null);
    Task<int> UpdateAsync(AlertEvent alertEvent, CancellationToken cancellationToken);
    Task<int> BulkUpdateAsync(IEnumerable<AlertEvent> alertEvents, CancellationToken cancellationToken);
    Task<int> CountUnresolvedAsync(CancellationToken cancellationToken);
}
