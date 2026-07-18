using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;
using System.Data;
using System.Data.Common;

namespace Infrastructure.Persistence;

internal class UnitOfWork : IUnitOfWork
{
    private readonly ILogger<UnitOfWork> logger;
    private readonly IDbConnection connection;
    private IDbTransaction? transaction;
    private bool disposed;

    public UnitOfWork(
        IDbConnectionFactory factory,
        ILogger<UnitOfWork> logger)
    {
        connection = factory.Create();
        this.logger = logger ?? throw new ArgumentNullException(nameof(logger));

        Users = new Lazy<IUserRepository>(() => new UserRepository(connection, GetTransaction));
        Teams = new Lazy<ITeamRepository>(() => new TeamRepository(connection, GetTransaction));
        Roles = new Lazy<IRoleRepository>(() => new RoleRepository(connection, GetTransaction));
        Actors = new Lazy<IActorRepository>(() => new ActorRepository(connection, GetTransaction));
        ResourceAccesses = new Lazy<IResourceAccessRepository>(() => new ResourceAccessRepository(connection, GetTransaction));
        Images = new Lazy<IImageRepository>(() => new ImageRepository(connection, GetTransaction));
        Stacks = new Lazy<IStackRepository>(() => new StackRepository(connection, GetTransaction));
        Platforms = new Lazy<IPlatformRepository>(() => new PlatformRepository(connection, GetTransaction));
        EdgeAgents = new Lazy<IEdgeAgentRepository>(() => new EdgeAgentRepository(connection, GetTransaction));
        Registries = new Lazy<IRegistryRepository>(() => new RegistryRepository(connection, GetTransaction));
        Containers = new Lazy<IContainerRepository>(() => new ContainerRepository(connection, GetTransaction));
        AlertRules = new Lazy<IAlertRuleRepository>(() => new AlertRuleRepository(connection, GetTransaction));
        AlertEvents = new Lazy<IAlertEventRepository>(() => new AlertEventRepository(connection, GetTransaction));
        Deployments = new Lazy<IDeploymentRepository>(() => new DeploymentRepository(connection, GetTransaction));
        RefreshTokens = new Lazy<IRefreshTokenRepository>(() => new RefreshTokenRepository(connection, GetTransaction));
        UserMfa = new Lazy<IUserMfaRepository>(() => new UserMfaRepository(connection, GetTransaction));
        MfaChallenges = new Lazy<IMfaChallengeRepository>(() => new MfaChallengeRepository(connection, GetTransaction));
        UserPreferences = new Lazy<IUserPreferencesRepository>(() => new UserPreferencesRepository(connection, GetTransaction));
        InstanceIdentity = new Lazy<IInstanceIdentityRepository>(() => new InstanceIdentityRepository(connection, GetTransaction));
        InstalledLicense = new Lazy<IInstalledLicenseRepository>(() => new InstalledLicenseRepository(connection, GetTransaction));
        LicenseUsage = new Lazy<ILicenseUsageRepository>(() => new LicenseUsageRepository(connection, GetTransaction));
        PlatformStats = new Lazy<IPlatformStatRepository>(() => new PlatformStatRepository(connection, GetTransaction));
        ContainerStats = new Lazy<IContainerStatRepository>(() => new ContainerStatRepository(connection, GetTransaction));
        ActivityEvents = new Lazy<IActivityEventRepository>(() => new ActivityEventRepository(connection, GetTransaction));
        GitAccounts = new Lazy<IGitAccountRepository>(() => new GitAccountRepository(connection, GetTransaction));
        GitRepositories = new Lazy<IGitReposRepository>(() => new GitReposRepository(connection, GetTransaction));
        ResourceBindings = new Lazy<IResourceBindingRepository>(() => new ResourceBindingRepository(connection, GetTransaction));
        SecretDefinitions = new Lazy<ISecretDefinitionRepository>(() => new SecretDefinitionRepository(connection, GetTransaction));
        SecretProviders = new Lazy<ISecretProviderRepository>(() => new SecretProviderRepository(connection, GetTransaction));
        Tags = new Lazy<ITagRepository>(() => new TagRepository(connection, GetTransaction));
        ResourceTags = new Lazy<IResourceTagRepository>(() => new ResourceTagRepository(connection, GetTransaction));
        OidcProviders = new Lazy<IOidcProviderRepository>(() => new OidcProviderRepository(connection, GetTransaction));
        OidcLoginStates = new Lazy<IOidcLoginStateRepository>(() => new OidcLoginStateRepository(connection, GetTransaction));
        OidcExternalLogins = new Lazy<IOidcExternalLoginRepository>(() => new OidcExternalLoginRepository(connection, GetTransaction));
        AutomationActions = new Lazy<IAutomationActionRepository>(() => new AutomationActionRepository(connection, GetTransaction));
        ActionRuns = new Lazy<IActionRunRepository>(() => new ActionRunRepository(connection, GetTransaction));
        BackupRepositories = new Lazy<IBackupRepositoryRepository>(() => new BackupRepositoryRepository(connection, GetTransaction));
        BackupRepositoryValidations = new Lazy<IBackupRepositoryValidationRepository>(() => new BackupRepositoryValidationRepository(connection, GetTransaction));
        BackupPolicies = new Lazy<IBackupPolicyRepository>(() => new BackupPolicyRepository(connection, GetTransaction));
        BackupRuns = new Lazy<IBackupRunRepository>(() => new BackupRunRepository(connection, GetTransaction));
        BackupRunItems = new Lazy<IBackupRunItemRepository>(() => new BackupRunItemRepository(connection, GetTransaction));
        BackupRunLogs = new Lazy<IBackupRunLogRepository>(() => new BackupRunLogRepository(connection, GetTransaction));
        BackupRestoreRuns = new Lazy<IBackupRestoreRunRepository>(() => new BackupRestoreRunRepository(connection, GetTransaction));
        BackupRestoreRunLogs = new Lazy<IBackupRestoreRunLogRepository>(() => new BackupRestoreRunLogRepository(connection, GetTransaction));
        BackupRepositoryLeases = new Lazy<IBackupRepositoryLeaseRepository>(() => new BackupRepositoryLeaseRepository(connection, GetTransaction));
        BackupSourceLeases = new Lazy<IBackupSourceLeaseRepository>(() => new BackupSourceLeaseRepository(connection, GetTransaction));
        BuildProjects = new Lazy<IBuildProjectRepository>(() => new BuildProjectRepository(connection, GetTransaction));
        BuildRuns = new Lazy<IBuildRunRepository>(() => new BuildRunRepository(connection, GetTransaction));
        BuildRunLogs = new Lazy<IBuildRunLogRepository>(() => new BuildRunLogRepository(connection, GetTransaction));
    }

    private Lazy<IUserRepository> Users { get; }
    private Lazy<ITeamRepository> Teams { get; }
    private Lazy<IRoleRepository> Roles { get; }
    private Lazy<IImageRepository> Images { get; }
    private Lazy<IActorRepository> Actors { get; }
    private Lazy<IResourceAccessRepository> ResourceAccesses { get; }
    private Lazy<IStackRepository> Stacks { get; }
    private Lazy<IPlatformRepository> Platforms { get; }
    private Lazy<IEdgeAgentRepository> EdgeAgents { get; }
    private Lazy<IRegistryRepository> Registries { get; }
    private Lazy<IContainerRepository> Containers { get; }
    private Lazy<IAlertRuleRepository> AlertRules { get; }
    private Lazy<IDeploymentRepository> Deployments { get; }
    private Lazy<IAlertEventRepository> AlertEvents { get; }
    private Lazy<IGitAccountRepository> GitAccounts { get; }
    private Lazy<IGitReposRepository> GitRepositories { get; }
    private Lazy<IResourceBindingRepository> ResourceBindings { get; }
    private Lazy<ISecretDefinitionRepository> SecretDefinitions { get; }
    private Lazy<ISecretProviderRepository> SecretProviders { get; }
    private Lazy<ITagRepository> Tags { get; }
    private Lazy<IResourceTagRepository> ResourceTags { get; }
    private Lazy<IOidcProviderRepository> OidcProviders { get; }
    private Lazy<IOidcLoginStateRepository> OidcLoginStates { get; }
    private Lazy<IOidcExternalLoginRepository> OidcExternalLogins { get; }
    private Lazy<IAutomationActionRepository> AutomationActions { get; }
    private Lazy<IActionRunRepository> ActionRuns { get; }
    private Lazy<IBackupRepositoryRepository> BackupRepositories { get; }
    private Lazy<IBackupRepositoryValidationRepository> BackupRepositoryValidations { get; }
    private Lazy<IBackupPolicyRepository> BackupPolicies { get; }
    private Lazy<IBackupRunRepository> BackupRuns { get; }
    private Lazy<IBackupRunItemRepository> BackupRunItems { get; }
    private Lazy<IBackupRunLogRepository> BackupRunLogs { get; }
    private Lazy<IBackupRestoreRunRepository> BackupRestoreRuns { get; }
    private Lazy<IBackupRestoreRunLogRepository> BackupRestoreRunLogs { get; }
    private Lazy<IBackupRepositoryLeaseRepository> BackupRepositoryLeases { get; }
    private Lazy<IBackupSourceLeaseRepository> BackupSourceLeases { get; }
    private Lazy<IBuildProjectRepository> BuildProjects { get; }
    private Lazy<IBuildRunRepository> BuildRuns { get; }
    private Lazy<IBuildRunLogRepository> BuildRunLogs { get; }
    private Lazy<IRefreshTokenRepository> RefreshTokens { get; }
    private Lazy<IUserMfaRepository> UserMfa { get; }
    private Lazy<IMfaChallengeRepository> MfaChallenges { get; }
    private Lazy<IUserPreferencesRepository> UserPreferences { get; }
    private Lazy<IInstanceIdentityRepository> InstanceIdentity { get; }
    private Lazy<IInstalledLicenseRepository> InstalledLicense { get; }
    private Lazy<ILicenseUsageRepository> LicenseUsage { get; }
    private Lazy<IPlatformStatRepository> PlatformStats { get; }
    private Lazy<IActivityEventRepository> ActivityEvents { get; }
    private Lazy<IContainerStatRepository> ContainerStats { get; }

    IUserRepository IUnitOfWork.Users => Users.Value;
    ITeamRepository IUnitOfWork.Teams => Teams.Value;
    IRoleRepository IUnitOfWork.Roles => Roles.Value;
    IImageRepository IUnitOfWork.Images => Images.Value;
    IActorRepository IUnitOfWork.Actors => Actors.Value;
    IResourceAccessRepository IUnitOfWork.ResourceAccesses => ResourceAccesses.Value;
    IStackRepository IUnitOfWork.Stacks => Stacks.Value;
    IPlatformRepository IUnitOfWork.Platforms => Platforms.Value;
    IEdgeAgentRepository IUnitOfWork.EdgeAgents => EdgeAgents.Value;
    IRegistryRepository IUnitOfWork.Registries => Registries.Value;
    IContainerRepository IUnitOfWork.Containers => Containers.Value;
    IAlertRuleRepository IUnitOfWork.AlertRules => AlertRules.Value;
    IAlertEventRepository IUnitOfWork.AlertEvents => AlertEvents.Value;
    IDeploymentRepository IUnitOfWork.Deployments => Deployments.Value;
    IPlatformStatRepository IUnitOfWork.PlatformStats => PlatformStats.Value;
    IRefreshTokenRepository IUnitOfWork.RefreshTokens => RefreshTokens.Value;
    IUserMfaRepository IUnitOfWork.UserMfa => UserMfa.Value;
    IMfaChallengeRepository IUnitOfWork.MfaChallenges => MfaChallenges.Value;
    IUserPreferencesRepository IUnitOfWork.UserPreferences => UserPreferences.Value;
    IInstanceIdentityRepository IUnitOfWork.InstanceIdentity => InstanceIdentity.Value;
    IInstalledLicenseRepository IUnitOfWork.InstalledLicense => InstalledLicense.Value;
    ILicenseUsageRepository IUnitOfWork.LicenseUsage => LicenseUsage.Value;
    IGitAccountRepository IUnitOfWork.GitAccounts => GitAccounts.Value;
    IGitReposRepository IUnitOfWork.GitRepositories => GitRepositories.Value;
    IContainerStatRepository IUnitOfWork.ContainerStats => ContainerStats.Value;
    IActivityEventRepository IUnitOfWork.ActivityEventRepository => ActivityEvents.Value;
    IResourceBindingRepository IUnitOfWork.ResourceBindings => ResourceBindings.Value;
    ISecretDefinitionRepository IUnitOfWork.SecretDefinitions => SecretDefinitions.Value;
    ISecretProviderRepository IUnitOfWork.SecretProviders => SecretProviders.Value;
    ITagRepository IUnitOfWork.Tags => Tags.Value;
    IResourceTagRepository IUnitOfWork.ResourceTags => ResourceTags.Value;
    IOidcProviderRepository IUnitOfWork.OidcProviders => OidcProviders.Value;
    IOidcLoginStateRepository IUnitOfWork.OidcLoginStates => OidcLoginStates.Value;
    IOidcExternalLoginRepository IUnitOfWork.OidcExternalLogins => OidcExternalLogins.Value;
    IAutomationActionRepository IUnitOfWork.AutomationActions => AutomationActions.Value;
    IActionRunRepository IUnitOfWork.ActionRuns => ActionRuns.Value;
    IBackupRepositoryRepository IUnitOfWork.BackupRepositories => BackupRepositories.Value;
    IBackupRepositoryValidationRepository IUnitOfWork.BackupRepositoryValidations => BackupRepositoryValidations.Value;
    IBackupPolicyRepository IUnitOfWork.BackupPolicies => BackupPolicies.Value;
    IBackupRunRepository IUnitOfWork.BackupRuns => BackupRuns.Value;
    IBackupRunItemRepository IUnitOfWork.BackupRunItems => BackupRunItems.Value;
    IBackupRunLogRepository IUnitOfWork.BackupRunLogs => BackupRunLogs.Value;
    IBackupRestoreRunRepository IUnitOfWork.BackupRestoreRuns => BackupRestoreRuns.Value;
    IBackupRestoreRunLogRepository IUnitOfWork.BackupRestoreRunLogs => BackupRestoreRunLogs.Value;
    IBackupRepositoryLeaseRepository IUnitOfWork.BackupRepositoryLeases => BackupRepositoryLeases.Value;
    IBackupSourceLeaseRepository IUnitOfWork.BackupSourceLeases => BackupSourceLeases.Value;
    IBuildProjectRepository IUnitOfWork.BuildProjects => BuildProjects.Value;
    IBuildRunRepository IUnitOfWork.BuildRuns => BuildRuns.Value;
    IBuildRunLogRepository IUnitOfWork.BuildRunLogs => BuildRunLogs.Value;

    // Lazily creates a transaction
    private IDbTransaction GetTransaction()
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        if (connection.State != ConnectionState.Open)
        {
            connection.Open();
        }
        transaction ??= connection.BeginTransaction();
        return transaction;
    }

    public async Task CommitAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);

        if (transaction == null) return;

        try
        {
            if (transaction is DbTransaction dbTransaction)
            {
                await dbTransaction.CommitAsync(cancellationToken);
            }
            else
            {
                transaction.Commit();
            }
        }
        catch
        {
            await RollbackAsync();
            throw;
        }
        finally
        {
            await DisposeTransactionAsync();
        }
    }

    public async Task RollbackAsync()
    {
        if (disposed || transaction == null) return;

        try
        {
            if (connection.State == ConnectionState.Open)
            {
                if (transaction is DbTransaction dbTransaction)
                {
                    await dbTransaction.RollbackAsync();
                }
                else
                {
                    transaction.Rollback();
                }
            }
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Rollback failed, transaction may already be aborted.");
        }
        finally
        {
            await DisposeTransactionAsync();
        }
    }

    private async Task DisposeTransactionAsync()
    {
        if (transaction == null) return;

        if (transaction is IAsyncDisposable asyncDisposable)
        {
            await asyncDisposable.DisposeAsync();
        }
        else
        {
            transaction.Dispose();
        }

        transaction = null;
    }

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;

        if (transaction != null)
        {
            await RollbackAsync();
        }

        if (connection is IAsyncDisposable asyncConn)
        {
            await asyncConn.DisposeAsync();
        }
        else
        {
            connection.Dispose();
        }

        disposed = true;
        GC.SuppressFinalize(this);
    }
}
