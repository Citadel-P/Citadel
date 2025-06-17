using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Identity;
using EFCore.BulkExtensions;
using Infrastructure.EntityFramework;
using Microsoft.EntityFrameworkCore;

namespace Infrastructure.Repositories;

internal class UnitOfWork : IUnitOfWork
{
    private readonly ApplicationDbContext dbContext;

    public UnitOfWork(ApplicationDbContext applicationDbContext)
    {
        dbContext = applicationDbContext;

        Users = new Lazy<IUserRepository>(() => new UserRepository(dbContext));
        Teams = new Lazy<ITeamRepository>(() => new TeamRepository(dbContext));
        Platforms = new Lazy<IPlatformRepository>(() => new PlatformRepository(dbContext));
        Registries = new Lazy<IRegistryRepository>(() => new RegistryRepository(dbContext));
        Containers = new Lazy<IContainerRepository>(() => new ContainerRepository(dbContext));
        RefreshTokens = new Lazy<IRefreshTokenRepository>(() => new RefreshTokenRepository(dbContext));
        ContainerStats = new Lazy<IContainerStatRepository>(() => new ContainerStatRepository(dbContext));
        PlatformStats = new Lazy<IPlatformStatRepository>(() => new PlatformStatRepository(dbContext));
    }

    public Lazy<IUserRepository> Users { get; }
    public Lazy<ITeamRepository> Teams { get; }
    public Lazy<IPlatformRepository> Platforms { get; }
    public Lazy<IContainerRepository> Containers { get; }
    public Lazy<IRegistryRepository> Registries { get; }
    public Lazy<IRefreshTokenRepository> RefreshTokens { get; }
    public Lazy<IContainerStatRepository> ContainerStats { get; }
    public Lazy<IPlatformStatRepository> PlatformStats { get; }

    // Public accessors
    IUserRepository IUnitOfWork.Users => Users.Value;
    IRegistryRepository IUnitOfWork.Registries => Registries.Value;
    IContainerRepository IUnitOfWork.Containers => Containers.Value;
    IPlatformRepository IUnitOfWork.Platforms => Platforms.Value;
    IRefreshTokenRepository IUnitOfWork.RefreshTokens => RefreshTokens.Value;
    IContainerStatRepository IUnitOfWork.ContainerStats => ContainerStats.Value;
    IPlatformStatRepository IUnitOfWork.PlatformStats => PlatformStats.Value;
    ITeamRepository IUnitOfWork.Teams => Teams.Value;

    public Task BulkInsertAsync<TEntity>(IEnumerable<TEntity> entities, CancellationToken cancellationToken = default) where TEntity : class
        => dbContext.BulkInsertAsync(entities, cancellationToken: cancellationToken);

    public Task<int> SaveChangesAsync(CancellationToken cancellationToken = default)
        => dbContext.SaveChangesAsync(cancellationToken);
}

internal class UserRepository(ApplicationDbContext db) : Repository<User>(db), IUserRepository { }
internal class RegistryRepository(ApplicationDbContext db) : Repository<Registry>(db), IRegistryRepository { }
internal class ContainerRepository(ApplicationDbContext db) : Repository<Container>(db), IContainerRepository { }
internal class ContainerStatRepository(ApplicationDbContext db) : Repository<ContainerStat>(db), IContainerStatRepository { }
internal class RefreshTokenRepository(ApplicationDbContext db) : Repository<RefreshToken>(db), IRefreshTokenRepository { }
internal class PlatformStatRepository(ApplicationDbContext db) : Repository<PlatformStat>(db), IPlatformStatRepository { }
internal class  TeamRepository(ApplicationDbContext db) : Repository<Team>(db), ITeamRepository { }
internal class PlatformRepository(ApplicationDbContext db) : Repository<Platform>(db), IPlatformRepository 
{
    public async Task<(string, PlatformConnectorType)> GetPlatformInfoAsync(Guid platformId, CancellationToken cancellationToken)
    {
        var platform = await db.Platforms
            .AsNoTracking()
            .Where(s => s.Id == platformId)
            .Select(s => new { s.Address, s.ConnectorType })
            .FirstOrDefaultAsync(cancellationToken);

        if (platform == null) return (string.Empty, PlatformConnectorType.Unknown);
        return (platform.Address, platform.ConnectorType);
    }
}
