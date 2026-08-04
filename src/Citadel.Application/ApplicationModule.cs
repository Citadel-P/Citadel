using Application.Permissions;
using Application.Features.Identity.Mfa.Services;
using Application.Features.Identity.Setup;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.Backups;
using Application.Services.Builds;
using Application.Services.Identity;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Pipelines;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using System.Threading.Channels;

namespace Application;

/// <summary>
/// The Application module.
/// </summary>
public static class ApplicationModule
{
    public static IServiceCollection RegisterApplicationModule(this IServiceCollection services)
    {
        services
            .AddServices()
            .AddBackgroundTasks()
            .AddSignalRServices()
            .AddMediator(options =>
            {
                options.ServiceLifetime = ServiceLifetime.Scoped;
                options.PipelineBehaviors =
                [
                    typeof(PermissionBehavior<,>),
                    typeof(ValidatorBehavior<,>)
                ];
                options.StreamPipelineBehaviors =
                [
                    typeof(StreamPermissionBehavior<,>)
                ];
            })
            .AddLookups()
            .AddPermissions()
            .AddSingleton<IErrorFactoryProvider, ErrorFactoryProvider>()
            .AddSingleton<IValidatorMetadataProvider, ValidatorMetadataProvider>();

        return services;
    }

    private static IServiceCollection AddServices(this IServiceCollection services)
    {
        services
            .AddMemoryCache()
            .AddSingleton(TimeProvider.System)
            .AddSingleton<IRoleCache, RoleCache>()
            .AddSingleton<ISetupStateCache, SetupStateCache>()
            .AddSingleton<IJwtService, JwtService>()
            .AddSingleton<ISyncBarrier, SyncBarrier>()
            .AddAlertEvaluators()
            .AddSingleton<AlertRuleCache>()
            .AddSingleton<IAlertService, AlertService>()
            .AddSingleton<IRepoCacheManager, RepoCacheManager>()
            .AddSingleton<IGitRepositoryPathNormalizer, GitRepositoryPathNormalizer>()
            .AddSingleton<ImageDigestCache>()
            .AddSingleton<IImageCheckBuilder, ImageCheckBuilder>()
            .AddSingleton<IImageDigestScanner, ImageDigestScanner>()
            .AddSingleton<DeploymentUpdateEvaluator>()
            .AddSingleton<ManualStackUpdateEvaluator>()
            .AddSingleton<IManualStackDeployedImageResolver, ManualStackDeployedImageResolver>()
            .AddSingleton<IUpdateCheckLeaseManager, UpdateCheckLeaseManager>()
            .AddSingleton<IImageScanScheduler, ImageScanScheduler>()
            .AddSingleton<IApplyDeploymentService, ApplyDeploymentService>()
            .AddSingleton<IAlertRuleProvider>(sp => sp.GetRequiredService<AlertRuleCache>())
            .AddSingleton<IContainerProcessingService, ContainerProcessingService>()
            .AddSingleton<IDeploymentProcessingService, DeploymentProcessingService>()
            .AddSingleton<IPlatformContainerCache, PlatformContainerCache>()
            .AddSingleton<IVolumePathNormalizer, VolumePathNormalizer>()
            .AddSingleton<IVolumeHelperImageResolver, VolumeHelperImageResolver>()
            .AddSingleton<IAgentRuntimeImageResolver, AgentRuntimeImageResolver>()
            .AddSingleton<IVolumeContentService, VolumeContentService>()
            .AddSingleton<IContainerEventBroadcaster, ContainerEventBroadcaster>()
            .AddSingleton<IPlatformHealthBroadCaster, PlatformHealthBroadCaster>()
            .AddSingleton<IContainerStatsBroadcaster, ContainerStatsBroadcaster>()
            .AddSingleton<IPlatformHealthMonitorJob, PlatformHealthMonitorJob>()
            .AddSingleton<GitRepoSyncInFlightTracker>()
            .AddSingleton<StackWebhookDeployQueueService>()
            .AddSingleton<StackWebhookDeployProcessor>()
            .AddSingleton<SwarmReconciliationJob>()
            .AddSingleton<ISwarmReconciliationCoordinator>(provider => provider.GetRequiredService<SwarmReconciliationJob>())
            .AddScoped<IActorScopeEvictor, ActorScopeEvictor>()
            .AddScoped<IAdministratorGuard, AdministratorGuard>()
            .AddScoped<IActorScopeProvider, ActorScopeProvider>()
            .AddSingleton<ICitadelPasswordHasher, CitadelPasswordHasher>()
            .AddSingleton<IPermissionCache, PermissionCache>()
            .AddScoped<GitHubConnectorStrategy>()
            .AddScoped<DockerHubConnectorStrategy>()
            .AddScoped<CustomRegistryConnectorStrategy>()
            .AddSingleton<IPullImageService, PullImageService>()
            .AddScoped<IRegistryConnectorResolver, RegistryConnectorResolver>()
            .AddScoped<IGitRepositoryContentService, GitRepositoryContentService>()
            .AddSingleton<IDelayWithJitterService, DelayWithJitterService>()
            .AddScoped<IPermissionService, PermissionService>()
            .AddScoped<IContainerAuthorizationService, ContainerAuthorizationService>()
            .AddScoped<ISignalRGroupAuthorizationService, SignalRGroupAuthorizationService>()
            .AddSingleton<IUserConnectionRevoker, UserConnectionRevoker>()
            .AddScoped<IActorRoleService, ActorRoleService>()
            .AddScoped<IActorResourceAccessService, ActorResourceAccessService>()
            .AddSingleton<INetworkService, NetworkService>()
            .AddScoped<IUserContextAccessor, UserContextAccessor>()
            .AddScoped<IRequestSessionMetadataAccessor, RequestSessionMetadataAccessor>()
            .AddScoped<IRefreshTokenCookieService, RefreshTokenCookieService>()
            .AddScoped<IMfaChallengeCookieService, MfaChallengeCookieService>()
            .AddScoped<IMfaSetupCookieService, MfaSetupCookieService>()
            .AddScoped<ICurrentRefreshSessionResolver, CurrentRefreshSessionResolver>()
            .AddSingleton<IStackDesiredStateProvider, StackDesiredStateProvider>()
            .AddSingleton<IStackRuntimeStateProvider, DockerStackRuntimeStateProvider>()
            .AddSingleton<IStackDriftChecker, StackDriftChecker>()
            .AddSingleton<IStackReconciler, StackReconciler>()
            .AddSingleton<IStackStoragePathProvider, StackStoragePathProvider>()
            .AddSingleton<IGitStackMaterializer, GitStackMaterializer>()
            .AddSingleton<ISecretValueProtector, SecretValueProtector>()
            .AddSingleton<IAdoptionFingerprintService, AdoptionFingerprintService>()
            .AddSingleton<ITotpService, TotpService>()
            .AddSingleton<IRecoveryCodeService, RecoveryCodeService>()
            .AddSingleton<IMfaPolicyService, MfaPolicyService>()
            .AddScoped<IAuthenticationSessionIssuer, AuthenticationSessionIssuer>()
            .AddScoped<ILocalAuthenticationCompletionService, LocalAuthenticationCompletionService>()
            .AddScoped<InitialAdministratorService>()
            .AddSingleton<ISecretRedactor, SecretRedactor>()
            .AddSingleton<IOidcDiscoveryService, OidcDiscoveryService>()
            .AddSingleton<IOidcAuthenticationService, OidcAuthenticationService>()
            .AddSingleton<ILicensePublicKeyRegistry, EmbeddedLicensePublicKeyRegistry>()
            .AddSingleton<ILicenseVerifier, LicenseVerifier>()
            .AddSingleton<ILicenseStateProvider, LicenseStateProvider>()
            .AddSingleton<ILicenseEntitlementService, LicenseEntitlementService>()
            .AddSingleton<IEdgeAgentManagementService, EdgeAgentManagementService>()
            .AddSingleton<IAgentHubPublicKeyProvider, AgentHubPublicKeyProvider>()
            .AddSingleton<IResourceBindingResolver, ResourceBindingResolver>()
            .AddScoped<IAutomationRunQueueService, AutomationRunQueueService>()
            .AddScoped<IAutomationExecutionService, AutomationExecutionService>()
            .AddSingleton<IAutomationRunCoordinator, AutomationRunCoordinator>()
            .AddSingleton<IResticEnvironmentBuilder, ResticEnvironmentBuilder>()
            .AddSingleton<IBackupRepositoryDestinationService, BackupRepositoryDestinationService>()
            .AddSingleton<IPlatformResticRunner, PlatformResticRunner>()
            .AddSingleton<ICitadelRecoveryAssetProvider, CitadelRecoveryAssetProvider>()
            .AddSingleton<ICitadelSystemBackupBuilder, CitadelSystemBackupBuilder>()
            .AddSingleton<IStackBackupVolumeResolver, StackBackupVolumeResolver>()
            .AddSingleton<IDeploymentBackupVolumeResolver, DeploymentBackupVolumeResolver>()
            .AddSingleton<IAutomationActionScheduler, AutomationActionScheduler>()
            .AddSingleton<IBackupPolicyScheduler, BackupPolicyScheduler>()
            .AddSingleton<IBackupRunCoordinator, BackupRunCoordinator>()
            .AddSingleton<IBackupRunExecutionService, BackupRunExecutionService>()
            .AddSingleton<IBackupRestoreRunCoordinator, BackupRestoreRunCoordinator>()
            .AddSingleton<IBackupRestoreRunExecutionService, BackupRestoreRunExecutionService>()
            .AddSingleton<IBuildRunCoordinator, BuildRunCoordinator>()
            .AddSingleton<IBuildRunCleanupService, BuildRunCleanupService>()
            .AddSingleton<IBuildImageResolver, BuildImageResolver>()
            .AddSingleton<IStackBuildImageBindingResolver, StackBuildImageBindingResolver>()
            .AddSingleton<IStackOperationBarrier, NoOpStackOperationBarrier>()
            .AddScoped<IBuildAgentPoolValidationService, BuildAgentPoolValidationService>()
            .AddScoped<IBuildRunRetentionService, BuildRunRetentionService>()
            .AddScoped<IBuildRunExecutionService, BuildRunExecutionService>()
            .AddSingleton<IApplyStackService, ApplyStackService>();

        services
            .AddSingleton(Channel.CreateBounded<ContainersStatBatch>(Helpers.ChannelDefaultOptions(singleWriter: false)))
            .AddSingleton(provider => provider.GetRequiredService<Channel<ContainersStatBatch>>().Writer)
            .AddSingleton(provider => provider.GetRequiredService<Channel<ContainersStatBatch>>().Reader)
            .AddSingleton(Channel.CreateBounded<GitRepoSyncRequest>(Helpers.ChannelDefaultOptions(singleWriter: false)))
            .AddSingleton(provider => provider.GetRequiredService<Channel<GitRepoSyncRequest>>().Writer)
            .AddSingleton(provider => provider.GetRequiredService<Channel<GitRepoSyncRequest>>().Reader)
            .AddSingleton(Channel.CreateBounded<StackWebhookDeploySignal>(Helpers.ChannelDefaultOptions(singleWriter: false)))
            .AddSingleton(provider => provider.GetRequiredService<Channel<StackWebhookDeploySignal>>().Writer)
            .AddSingleton(provider => provider.GetRequiredService<Channel<StackWebhookDeploySignal>>().Reader)
            .AddSingleton(Channel.CreateBounded<UnmanagedContainerAlertRequest>(Helpers.ChannelDefaultOptions(singleWriter: false)))
            .AddSingleton(provider => provider.GetRequiredService<Channel<UnmanagedContainerAlertRequest>>().Writer)
            .AddSingleton(provider => provider.GetRequiredService<Channel<UnmanagedContainerAlertRequest>>().Reader)
            .AddSingleton(Channel.CreateBounded<(Guid Id, PlatformStatsResult Stats)>(Helpers.ChannelDefaultOptions(singleWriter: false)))
            .AddSingleton(provider => provider.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Writer)
            .AddSingleton(provider => provider.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Reader);

        services.TryAddSingleton<IAutomationApiEndpointCatalog, EmptyAutomationApiEndpointCatalog>();

        return services;
    }

    private static IServiceCollection AddLookups(this IServiceCollection services)
        => services
            ;

    private static IServiceCollection AddSignalRServices(this IServiceCollection services) =>
        services
            .AddSingleton<IStreamSubscriptionResolver, StreamSubscriptionResolver>()
            .AddSingleton<ContainerInfoStreamManager>()
            .AddSingleton<ContainerLogStreamManager>()
            .AddSingleton<StackInfoStreamManager>()
            .AddSingleton<StackLogStreamManager>()
            .AddSingleton<StackStreamManager>()
            .AddSingleton<DeploymentStreamManager>()
            .AddSingleton<ActivityStreamManager>()
            .AddSingleton<AlertEventStreamManager>()
            .AddSingleton<ExecSessionManager>()
            .AddSingleton<ImageStreamManager>()
            .AddSingleton<PlatformStreamManager>()
            .AddSingleton<ContainerStreamManager>()
            .AddSingleton<DockerDaemonStreamManager>()
            .AddSingleton<GitRepositoryStreamManager>()
            .AddSingleton<BackupRepositoryStreamManager>()
            .AddSingleton<BackupPolicyStreamManager>()
            .AddSingleton<BackupRunStreamManager>()
            .AddSingleton<BackupRestoreRunStreamManager>()
            .AddSingleton<AutomationActionStreamManager>()
            .AddSingleton<IImageStreamManager>(s => s.GetRequiredService<ImageStreamManager>())
            .AddSingleton<IExecSessionManager>(s => s.GetRequiredService<ExecSessionManager>())
            .AddSingleton<IActivityStreamManager>(s => s.GetRequiredService<ActivityStreamManager>())
            .AddSingleton<IAlertEventStreamManager>(s => s.GetRequiredService<AlertEventStreamManager>())
            .AddSingleton<IPlatformStreamManager>(s => s.GetRequiredService<PlatformStreamManager>())
            .AddSingleton<IContainerStreamManager>(s => s.GetRequiredService<ContainerStreamManager>())
            .AddSingleton<IDeploymentStreamManager>(s => s.GetRequiredService<DeploymentStreamManager>())
            .AddSingleton<IDockerDaemonStreamManager>(s => s.GetRequiredService<DockerDaemonStreamManager>())
            .AddSingleton<IContainerLogStreamManager>(s => s.GetRequiredService<ContainerLogStreamManager>())
            .AddSingleton<IStackLogStreamManager>(s => s.GetRequiredService<StackLogStreamManager>())
            .AddSingleton<IGitRepositoryStreamManager>(s => s.GetRequiredService<GitRepositoryStreamManager>())
            .AddSingleton<IBackupRepositoryStreamManager>(s => s.GetRequiredService<BackupRepositoryStreamManager>())
            .AddSingleton<IBackupPolicyStreamManager>(s => s.GetRequiredService<BackupPolicyStreamManager>())
            .AddSingleton<IBackupRunStreamManager>(s => s.GetRequiredService<BackupRunStreamManager>())
            .AddSingleton<IBackupRestoreRunStreamManager>(s => s.GetRequiredService<BackupRestoreRunStreamManager>())
            .AddSingleton<BuildProjectStreamManager>()
            .AddSingleton<BuildRunStreamManager>()
            .AddSingleton<BuildAgentPoolStreamManager>()
            .AddSingleton<IBuildProjectStreamManager>(s => s.GetRequiredService<BuildProjectStreamManager>())
            .AddSingleton<IBuildRunStreamManager>(s => s.GetRequiredService<BuildRunStreamManager>())
            .AddSingleton<IBuildAgentPoolStreamManager>(s => s.GetRequiredService<BuildAgentPoolStreamManager>())
            .AddSingleton<IAutomationActionStreamManager>(s => s.GetRequiredService<AutomationActionStreamManager>())
            .AddSingleton<IStackStreamManager>(s => s.GetRequiredService<StackStreamManager>());

    private static IServiceCollection AddBackgroundTasks(this IServiceCollection services)
    {
        if (Helpers.IsDesignTime()) return services;

        services
            .AddHostedService<InitialAdministratorBootstrapService>()
            .AddHostedService<DockerDaemonEventJob>()
            .AddHostedService<CleanupJob>()
            .AddHostedService<PlatformSyncJob>()
            .AddHostedService<PlatformStatsStreamerJob>()
            .AddHostedService<ContainerStatsStreamerJob>()
            .AddHostedService<UnmanagedContainerAlertJob>()
            .AddHostedService<DeploymentImageScannerJob>()
            .AddHostedService<DeploymentAutoUpdateJob>()
            .AddHostedService<ManualStackAutoUpdateJob>()
            .AddHostedService<PlatformStatsWriterJob>()
            .AddHostedService<ContainerStatsWriterJob>()
            .AddHostedService<ContainerSyncJob>()
            .AddHostedService(provider => provider.GetRequiredService<SwarmReconciliationJob>())
            .AddHostedService<ImageSyncJob>()
            .AddHostedService<AlertRuleCacheWarmup>()
            .AddHostedService<ReconcilableResourceJob>()
            .AddHostedService<StackDriftMonitorJob>()
            .AddHostedService<GitRepositoryPollingJob>()
            .AddHostedService<GitRepoSyncJob>()
            .AddHostedService<StackWebhookDeployJob>()
            .AddHostedService<AutomationActionSchedulerJob>()
            .AddHostedService<AutomationActionRunWorkerJob>()
            .AddHostedService<BackupPolicySchedulerJob>()
            .AddHostedService<BackupRunWorkerJob>()
            .AddHostedService<BackupRestoreRunWorkerJob>()
            .AddHostedService<BuildRunWorkerJob>()
            .AddHostedService<BuildAgentPoolHealthMonitorJob>()
            .AddHostedService<LicenseTransitionMonitorJob>()
            .AddHostedService(s => s.GetRequiredService<IPlatformHealthMonitorJob>());

        return services;
    }

    private static IServiceCollection AddPermissions(this IServiceCollection services)
    {
        return
            services
            .AddScoped<IPermissionEvaluator, PermissionEvaluator>();
    }

}
