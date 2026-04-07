using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Logging;
using System.Data;

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
        Roles = new Lazy<IRoleRepository>(() => new RoleRepository(connection, GetTransaction));
        Actors = new Lazy<IActorRepository>(() => new ActorRepository(connection, GetTransaction));
        Images = new Lazy<IImageRepository>(() => new ImageRepository(connection, GetTransaction));
        Stacks = new Lazy<IStackRepository>(() => new StackRepository(connection, GetTransaction));
        Platforms = new Lazy<IPlatformRepository>(() => new PlatformRepository(connection, GetTransaction));
        Registries = new Lazy<IRegistryRepository>(() => new RegistryRepository(connection, GetTransaction));
        Containers = new Lazy<IContainerRepository>(() => new ContainerRepository(connection, GetTransaction));
        AlertRules = new Lazy<IAlertRuleRepository>(() => new AlertRuleRepository(connection, GetTransaction));
        AlertEvents = new Lazy<IAlertEventRepository>(() => new AlertEventRepository(connection, GetTransaction));
        Deployments = new Lazy<IDeploymentRepository>(() => new DeploymentRepository(connection, GetTransaction));
        RefreshTokens = new Lazy<IRefreshTokenRepository>(() => new RefreshTokenRepository(connection, GetTransaction));
        PlatformStats = new Lazy<IPlatformStatRepository>(() => new PlatformStatRepository(connection, GetTransaction));
        ContainerStats = new Lazy<IContainerStatRepository>(() => new ContainerStatRepository(connection, GetTransaction));
        ActivityEvents = new Lazy<IActivityEventRepository>(() => new ActivityEventRepository(connection, GetTransaction));
        GitAccounts = new Lazy<IGitAccountRepository>(() => new GitAccountRepository(connection, GetTransaction));
        GitRepositories = new Lazy<IGitReposRepository>(() => new GitReposRepository(connection, GetTransaction));
    }

    private Lazy<IUserRepository> Users { get; }
    private Lazy<ITeamRepository> Teams { get; }
    private Lazy<IRoleRepository> Roles { get; }
    private Lazy<IImageRepository> Images { get; }
    private Lazy<IActorRepository> Actors { get; }
    private Lazy<IStackRepository> Stacks { get; }
    private Lazy<IPlatformRepository> Platforms { get; }
    private Lazy<IRegistryRepository> Registries { get; }
    private Lazy<IContainerRepository> Containers { get; }
    private Lazy<IAlertRuleRepository> AlertRules { get; }
    private Lazy<IDeploymentRepository> Deployments { get; }
    private Lazy<IAlertEventRepository> AlertEvents { get; }
    private Lazy<IGitAccountRepository> GitAccounts { get; }
    private Lazy<IGitReposRepository> GitRepositories { get; }
    private Lazy<IRefreshTokenRepository> RefreshTokens { get; }
    private Lazy<IPlatformStatRepository> PlatformStats { get; }
    private Lazy<IActivityEventRepository> ActivityEvents { get; }
    private Lazy<IContainerStatRepository> ContainerStats { get; }

    IUserRepository IUnitOfWork.Users => Users.Value;
    ITeamRepository IUnitOfWork.Teams => Teams.Value;
    IRoleRepository IUnitOfWork.Roles => Roles.Value;
    IImageRepository IUnitOfWork.Images => Images.Value;
    IActorRepository IUnitOfWork.Actors => Actors.Value;
    IStackRepository IUnitOfWork.Stacks => Stacks.Value;
    IPlatformRepository IUnitOfWork.Platforms => Platforms.Value;
    IRegistryRepository IUnitOfWork.Registries => Registries.Value;
    IContainerRepository IUnitOfWork.Containers => Containers.Value;
    IAlertRuleRepository IUnitOfWork.AlertRules => AlertRules.Value;
    IAlertEventRepository IUnitOfWork.AlertEvents => AlertEvents.Value;
    IDeploymentRepository IUnitOfWork.Deployments => Deployments.Value;
    IPlatformStatRepository IUnitOfWork.PlatformStats => PlatformStats.Value;
    IRefreshTokenRepository IUnitOfWork.RefreshTokens => RefreshTokens.Value;
    IGitAccountRepository IUnitOfWork.GitAccounts => GitAccounts.Value;
    IGitReposRepository IUnitOfWork.GitRepositories => GitRepositories.Value;
    IContainerStatRepository IUnitOfWork.ContainerStats => ContainerStats.Value;
    IActivityEventRepository IUnitOfWork.ActivityEventRepository => ActivityEvents.Value;

    // Lazily creates a transaction
    private IDbTransaction GetTransaction()
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        transaction ??= connection.BeginTransaction();
        return transaction;
    }

    public async Task CommitAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        if (transaction == null) return;

        await DbRetryPolicies.RetryOnBusy.ExecuteAsync(async ct =>
        {
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
            catch (SqliteException ex) when (DbRetryPolicies.IsBusy(ex))
            {
                logger.LogWarning(ex, "SQLite busy/locked during commit. Will retry.");
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error committing transaction");
                await RollbackAsync();
                throw;
            }
        }, cancellationToken);
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
