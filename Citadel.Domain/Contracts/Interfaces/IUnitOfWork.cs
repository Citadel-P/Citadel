using Domain.Entities;
using Domain.Entities.Identity;

namespace Domain.Contracts.Interfaces;

public interface IUnitOfWork
{
    IUserRepository Users { get; }
    ITeamRepository Teams { get; }
    IRegistryRepository Registries { get; }
    IPlatformRepository Platforms { get; }
    IContainerRepository Containers { get; }
    IRefreshTokenRepository RefreshTokens { get; }
    IPlatformStatRepository PlatformStats { get; }
    IContainerStatRepository ContainerStats { get; }

    Task BulkInsertAsync<TEntity>(IEnumerable<TEntity> entities, CancellationToken cancellationToken = default) where TEntity : class;
    Task<int> SaveChangesAsync(CancellationToken ct = default);
}

public interface IUserRepository : IRepository<User> { }
public interface IRegistryRepository : IRepository<Registry> { }
public interface IContainerRepository : IRepository<Container> { }
public interface IContainerStatRepository : IRepository<ContainerStat> { }
public interface IRefreshTokenRepository : IRepository<RefreshToken> { }
public interface IPlatformStatRepository : IRepository<PlatformStat> { }
public interface ITeamRepository : IRepository<Team> { }
public interface IPlatformRepository : IRepository<Platform> 
{
    Task<(string address, PlatformConnectorType connectorType)> GetPlatformInfoAsync(Guid platformId, CancellationToken cancellationToken);
}
