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
        this.logger = logger ?? throw new ArgumentNullException(nameof(logger));

        Users = new Lazy<IUserRepository>(() => new UserRepository(connection, GetTransaction));
        Teams = new Lazy<ITeamRepository>(() => new TeamRepository(connection, GetTransaction));
        Images = new Lazy<IImageRepository>(() => new ImageRepository(connection, GetTransaction));
        Platforms = new Lazy<IPlatformRepository>(() => new PlatformRepository(connection, GetTransaction));
        Registries = new Lazy<IRegistryRepository>(() => new RegistryRepository(connection, GetTransaction));
        Containers = new Lazy<IContainerRepository>(() => new ContainerRepository(connection, GetTransaction));
        RefreshTokens = new Lazy<IRefreshTokenRepository>(() => new RefreshTokenRepository(connection, GetTransaction));
        PlatformStats = new Lazy<IPlatformStatRepository>(() => new PlatformStatRepository(connection, GetTransaction));
        ContainerStats = new Lazy<IContainerStatRepository>(() => new ContainerStatRepository(connection, GetTransaction));
    }

    private Lazy<IUserRepository> Users { get; }
    private Lazy<ITeamRepository> Teams { get; }
    private Lazy<IImageRepository> Images { get; }
    private Lazy<IPlatformRepository> Platforms { get; }
    private Lazy<IRegistryRepository> Registries { get; }
    private Lazy<IContainerRepository> Containers { get; }
    private Lazy<IRefreshTokenRepository> RefreshTokens { get; }
    private Lazy<IContainerStatRepository> ContainerStats { get; }
    private Lazy<IPlatformStatRepository> PlatformStats { get; }

    IUserRepository IUnitOfWork.Users => Users.Value;
    ITeamRepository IUnitOfWork.Teams => Teams.Value;
    IImageRepository IUnitOfWork.Images => Images.Value;
    IPlatformRepository IUnitOfWork.Platforms => Platforms.Value;
    IRegistryRepository IUnitOfWork.Registries => Registries.Value;
    IContainerRepository IUnitOfWork.Containers => Containers.Value;
    IRefreshTokenRepository IUnitOfWork.RefreshTokens => RefreshTokens.Value;
    IContainerStatRepository IUnitOfWork.ContainerStats => ContainerStats.Value;
    IPlatformStatRepository IUnitOfWork.PlatformStats => PlatformStats.Value;

    // Lazily creates a transaction - to prevent sqlite table locking
    private IDbTransaction GetTransaction()
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        transaction ??= connection.BeginTransaction();
        return transaction;
    }

    public async Task CommitAsync()
    {
        ObjectDisposedException.ThrowIf(disposed, this);
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
