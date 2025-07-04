using System.Data;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Infrastructure.Persistence;

internal class UnitOfWork : IUnitOfWork
{
    private readonly ILogger<UnitOfWork> logger;
    private readonly IDbConnection connection;
    private IDbTransaction? transaction;
    private bool disposed;

    public UnitOfWork(IDbConnectionFactory factory, ILogger<UnitOfWork> logger)
    {
        connection = factory.Create();
        transaction = connection.BeginTransaction();
        this.logger = logger ?? throw new ArgumentNullException(nameof(logger));

        Users = new Lazy<IUserRepository>(() => new UserRepository(connection, transaction));
        Teams = new Lazy<ITeamRepository>(() => new TeamRepository(connection, transaction));
        Platforms = new Lazy<IPlatformRepository>(() => new PlatformRepository(connection, transaction));
        Registries = new Lazy<IRegistryRepository>(() => new RegistryRepository(connection, transaction));
        Containers = new Lazy<IContainerRepository>(() => new ContainerRepository(connection, transaction));
        RefreshTokens = new Lazy<IRefreshTokenRepository>(() => new RefreshTokenRepository(connection, transaction));
        PlatformStats = new Lazy<IPlatformStatRepository>(() => new PlatformStatRepository(connection, transaction));
        ContainerStats = new Lazy<IContainerStatRepository>(() => new ContainerStatRepository(connection, transaction));
    }

    private Lazy<IUserRepository> Users { get; }
    private Lazy<ITeamRepository> Teams { get; }
    private Lazy<IPlatformRepository> Platforms { get; }
    private Lazy<IRegistryRepository> Registries { get; }
    private Lazy<IContainerRepository> Containers { get; }
    private Lazy<IRefreshTokenRepository> RefreshTokens { get; }
    private Lazy<IContainerStatRepository> ContainerStats { get; }
    private Lazy<IPlatformStatRepository> PlatformStats { get; }

    IUserRepository IUnitOfWork.Users => Users.Value;
    ITeamRepository IUnitOfWork.Teams => Teams.Value;
    IPlatformRepository IUnitOfWork.Platforms => Platforms.Value;
    IRegistryRepository IUnitOfWork.Registries => Registries.Value;
    IContainerRepository IUnitOfWork.Containers => Containers.Value;
    IRefreshTokenRepository IUnitOfWork.RefreshTokens => RefreshTokens.Value;
    IContainerStatRepository IUnitOfWork.ContainerStats => ContainerStats.Value;
    IPlatformStatRepository IUnitOfWork.PlatformStats => PlatformStats.Value;

    public async Task CommitAsync()
    {
        if (disposed) throw new ObjectDisposedException(nameof(UnitOfWork));
        if (transaction == null) return;

        try
        {
            if (transaction is IAsyncDisposable asyncDisposableTransaction)
            {
                transaction.Commit();
                await asyncDisposableTransaction.DisposeAsync();
            }
            else
            {
                transaction.Commit();
                transaction.Dispose();
            }
            transaction = null;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error committing transaction");
            await RollbackAsync();
            throw;
        }
    }

    public async Task RollbackAsync()
    {
        if (disposed || transaction == null) return;

        try
        {
            transaction.Rollback();
            if (transaction is IAsyncDisposable asyncDisposableTransaction)
            {
                await asyncDisposableTransaction.DisposeAsync();
            }
            else
            {
                transaction.Dispose();
            }
            transaction = null;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error rolling back transaction");
            throw;
        }
    }

    public async ValueTask DisposeAsync()
    {
        await DisposeAsync(true);
        GC.SuppressFinalize(this);
    }

    protected virtual async ValueTask DisposeAsync(bool disposing)
    {
        if (disposed) return;

        if (disposing)
        {
            try
            {
                if (transaction != null)
                {
                    if (transaction is IAsyncDisposable asyncDisposableTransaction)
                    {
                        await asyncDisposableTransaction.DisposeAsync();
                    }
                    else
                    {
                        transaction.Dispose();
                    }
                }

                if (connection.State != ConnectionState.Closed)
                {
                    if (connection is IAsyncDisposable asyncDisposableConnection)
                    {
                        await asyncDisposableConnection.DisposeAsync();
                    }
                    else
                    {
                        connection.Dispose();
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error disposing unit of work");
            }
        }
        disposed = true;
    }
}
