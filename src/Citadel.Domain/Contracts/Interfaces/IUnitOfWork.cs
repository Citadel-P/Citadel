using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Identity;
using Domain.Entities.Registries;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork : IAsyncDisposable
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IPlatformRepository Platforms { get; }
    IContainerRepository Containers { get; }
    IRegistryRepository Registries { get; }
    IRefreshTokenRepository RefreshTokens { get; }
    IContainerStatRepository ContainerStats { get; }
    IPlatformStatRepository PlatformStats { get; }

    Task CommitAsync();
    Task RollbackAsync();
}

public interface IUserRepository 
{
    Task<UserAuthInfo?> GetUserAuthInfoByEmailAsync(string email, CancellationToken cancellationToken);
}
public interface IRegistryRepository 
{
    Task<Registry?> GetAsync(Guid Id, CancellationToken cancellationToken);
    Task<IEnumerable<Registry>> GetAllAsync(CancellationToken cancellationToken);
    Task<bool> ExistsAsync(string name, CancellationToken cancellationToken);
    Task<bool> IsNameUsedByAnotherRegistryAsync(Guid id, string name, CancellationToken cancellationToken);
    Task<int> AddAsync(Registry registry, CancellationToken cancellationToken);
    Task<int> UpdateAsync(Registry registry, CancellationToken cancellationToken);
    Task<RegistryConfigurationBase?> GetRegistryConfigurationAsync(string name, CancellationToken cancellationToken);

    Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken);
}
public interface IContainerRepository 
{
    Task<Container?> GetByIdAsync(string containerId, CancellationToken cancellationToken);
    Task<ContainerInfo?> GetContainerInfoAsync(string containerId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>> GetByPlatformIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<Container>?> GetAllWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken);

    Task<int> AddAsync(Container container, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<Container> containers, CancellationToken cancellationToken);

    Task<int> UpdateContainerAsync(Container container, CancellationToken cancellationToken);
    Task<int> UpdateContainersStateAsync(IEnumerable<Guid> ids, ContainerStateStatus state, CancellationToken cancellationToken);

    Task<int> DeleteAsync(IEnumerable<Guid> containersId, CancellationToken cancellationToken);
}
public interface IContainerStatRepository 
{
    Task<IEnumerable<ContainerStat>> GetStatsAggregatedLast24HoursAsync(string containerId, CancellationToken cancellationToken);
    Task<int> BulkInsertAsync(IEnumerable<ContainerStat> stats, CancellationToken cancellationToken);
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
}
public interface ITeamRepository { }
public interface IPlatformRepository 
{
    Task<Platform?> GetByIdAsync(Guid platformId, CancellationToken cancellationToken);
    Task<Platform?> GetByNameAsync(string name, CancellationToken cancellationToken);
    Task<IEnumerable<Platform>?> GetPlatformsWithLatestStatAsync(CancellationToken cancellationToken);
    Task<Platform?> GetPlatformWithLatestStatAsync(Guid platformId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformDetailsByContainerIdAsync(string containerId, CancellationToken cancellationToken);
    Task<PlatformConnectionInfo?> GetPlatformInfoAsync(Guid platformId, CancellationToken cancellationToken);
    Task<IEnumerable<PlatformConnectionInfo>> GetPlatformsInfoAsync(CancellationToken cancellationToken);

    Task<int?> IsPlatformNameUniqueExceptForIdAsync(Guid platformId, string name, CancellationToken cancellationToken);
    Task<bool> NameOrAddressExistsAsync(string name, string address, CancellationToken cancellationToken);

    Task<int> AddPlatformAsync(Platform platform, CancellationToken cancellationToken);
    Task<int> UpdatePlatformAsync(Platform platform, CancellationToken cancellationToken);

    Task<int> DeleteAsync(Guid platformId, CancellationToken cancellationToken);
}
