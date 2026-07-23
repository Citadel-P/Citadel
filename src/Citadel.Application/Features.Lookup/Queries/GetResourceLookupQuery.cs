using Application.Features.Networks.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Lookup.Queries;

public sealed record GetResourceLookupQuery(
    LookupResourceType? SourceResourceType,
    Guid? SourceResourceId,
    LookupResourceType TargetResourceType,
    LookupContext Context) : IQuery<Result<IEnumerable<ResourceInfo>>>;

internal sealed class GetResourceLookupQueryHandler(
    INetworkService networkService,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory,
    IUserContextAccessor userContextAccessor,
    TimeProvider timeProvider,
    IUnitOfWork unitOfWork) : IQueryHandler<GetResourceLookupQuery, Result<IEnumerable<ResourceInfo>>>
{
    public async ValueTask<Result<IEnumerable<ResourceInfo>>> Handle(GetResourceLookupQuery query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        if (user is null || user.UserId == Guid.Empty)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("Invalid user ID."));
        }

        var hasSourceType = query.SourceResourceType.HasValue;
        var hasSourceId = query.SourceResourceId.HasValue;
        if (!hasSourceType && hasSourceId)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(
                new BadRequestError("sourceResourceType must be provided when sourceResourceId is specified."));
        }

        return await ResolveAsync(
            query.SourceResourceType,
            query.SourceResourceId,
            query.TargetResourceType,
            user.UserId,
            query.Context,
            cancellationToken);
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> ResolveAsync(
        LookupResourceType? sourceType,
        Guid? sourceId,
        LookupResourceType targetType,
        Guid userId,
        LookupContext context,
        CancellationToken cancellationToken)
    {
        if (sourceType.HasValue && sourceId.HasValue)
        {
            var accessResult = await ValidateSourceAccessAsync(sourceType.Value, sourceId.Value, userId, cancellationToken);
            if (accessResult.IsFailure(out var accessError))
            {
                return Result.Failure<IEnumerable<ResourceInfo>>(accessError);
            }
        }

        return (sourceType, targetType) switch
        {
            (LookupResourceType.Deployment, LookupResourceType.Platform) => await GetDeploymentPlatformLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Deployment, LookupResourceType.Registry) => await GetDeploymentRegistryLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Deployment, LookupResourceType.Image) => await GetDeploymentImageLookupAsync(sourceId, cancellationToken),
            (LookupResourceType.Deployment, LookupResourceType.Network) => await GetDeploymentNetworkLookupAsync(context, cancellationToken),
            (LookupResourceType.Deployment, LookupResourceType.ResourceBinding) => await GetResourceBindingLookupAsync(ResourceBindingScope.Deployment, sourceId, cancellationToken),
            (LookupResourceType.Stack, LookupResourceType.Platform) => await GetStackPlatformLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Stack, LookupResourceType.Registry) => await GetStackRegistryLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Stack, LookupResourceType.GitRepository) => await GetStackGitRepositoryLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Stack, LookupResourceType.ResourceBinding) => await GetResourceBindingLookupAsync(ResourceBindingScope.Stack, sourceId, cancellationToken),
            (LookupResourceType.User, LookupResourceType.Team) => await GetUserTeamLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.User, LookupResourceType.Role) => await GetUserRoleLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Deployment) => await GetPlatformDeploymentLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Stack) => await GetPlatformStackLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Registry) => await GetPlatformRegistryLookupAsync(sourceId, userId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Image) => await GetPlatformImageLookupAsync(sourceId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Network) => await GetPlatformNetworkLookupAsync(sourceId, cancellationToken),
            (LookupResourceType.Platform, LookupResourceType.Volume) => await GetPlatformVolumeLookupAsync(sourceId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.Platform) => await GetAlertPlatformLookupAsync(userId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.Deployment) => await GetAlertDeploymentLookupAsync(userId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.Stack) => await GetAlertStackLookupAsync(userId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.GitRepository) => await GetAlertGitRepositoryLookupAsync(userId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.AutomationAction) => await GetAlertAutomationActionLookupAsync(userId, cancellationToken),
            (LookupResourceType.Alert, LookupResourceType.AlertChannel) => await GetAlertChannelLookupAsync(userId, cancellationToken),
            (LookupResourceType.Image, LookupResourceType.Registry) => await GetImageRegistryLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Platform) => await GetPlatformLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Alert) => await GetAlertLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.User) => await GetUserLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.UserActor) => await GetUserActorLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Team) => await GetTeamLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Role) => await GetRoleLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Registry) => await GetRegistryLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.GitRepository) => await GetGitRepositoryLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.OidcProvider) => await GetOidcProviderLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.AutomationAction) => await GetAutomationActionLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.BackupRepository) => await GetBackupRepositoryLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.BackupPolicy) => await GetBackupPolicyLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Build) => await GetBuildLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.BuildAgentPool) => await GetBuildAgentPoolLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Deployment) => await GetDeploymentLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Stack) => await GetStackLookupAsync(userId, cancellationToken),
            (null, LookupResourceType.Image) => await GetImageLookupAsync(userId, context, cancellationToken),
            (null, LookupResourceType.ResourceBinding) => await GetGlobalResourceBindingLookupAsync(cancellationToken),
            (null, LookupResourceType.License) => await GetLicenseLookupAsync(cancellationToken),
            _ => Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError(GetUnsupportedLookupMessage(sourceType, targetType)))
        };
    }

    private async Task<Result> ValidateSourceAccessAsync(LookupResourceType sourceType, Guid sourceId, Guid userId, CancellationToken cancellationToken)
    {
        var canAccess = sourceType switch
        {
            LookupResourceType.Deployment => await unitOfWork.Deployments.CanAccessAsync(userId, sourceId, cancellationToken),
            LookupResourceType.Stack => await unitOfWork.Stacks.CanAccessAsync(userId, sourceId, cancellationToken),
            LookupResourceType.User => await unitOfWork.Users.CanAccessAsync(userId, sourceId, cancellationToken),
            LookupResourceType.Platform => await unitOfWork.Platforms.CanAccessAsync(userId, sourceId, cancellationToken),
            LookupResourceType.Alert => await unitOfWork.AlertRules.CanAccessAsync(userId, sourceId, cancellationToken),
            _ => false
        };

        return canAccess
            ? Result.Success()
            : Result.Failure(GetSourceNotFoundError(sourceType));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetDeploymentPlatformLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
     => sourceId.HasValue
         ? Result.Success(await unitOfWork.Deployments.GetPlatformLookupAsync(sourceId.Value, userId, cancellationToken))
         : Result.Success((await unitOfWork.Platforms.GetAuthorizedAsync(
                 userId,
                 ResourceType.Platform,
                 PermissionLevel.Read,
                 SpecificPermission.None,
                 cancellationToken))
             .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetDeploymentRegistryLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Deployments.GetRegistryLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Registries.GetAuthorizedAsync(userId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetDeploymentImageLookupAsync(Guid? sourceId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Deployments.GetImageLookupAsync(sourceId.Value, cancellationToken))
            : Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("sourceResourceId is required for Deployment -> Image lookup."));
    
    private async Task<Result<IEnumerable<ResourceInfo>>> GetDeploymentNetworkLookupAsync(LookupContext? context, CancellationToken cancellationToken)
    {
        if (context?.PlatformId == null || !context.PlatformId.HasValue)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("PlatformId is required for Deployment -> Network lookup."));
        }
        var result = await networkService.List(new ListNetworks(context.PlatformId.Value), cancellationToken);
        if (result.IsFailure(out var error, out var networks))
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(error.Message);
        }
        return Result.Success(networks.Select(static item => new ResourceInfo(Guid.Empty, item.Name)));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetStackPlatformLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Stacks.GetPlatformLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Platforms.GetAuthorizedAsync(
                 userId,
                 ResourceType.Platform,
                 PermissionLevel.Read,
                 SpecificPermission.None,
                 cancellationToken))
             .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetStackRegistryLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Stacks.GetRegistryLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Registries.GetAuthorizedAsync(userId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetStackGitRepositoryLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Stacks.GetGitRepositoryLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.GitRepositories.GetAuthorizedAsync(userId, ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetResourceBindingLookupAsync(
        ResourceBindingScope scope,
        Guid? sourceId,
        CancellationToken cancellationToken)
    {
        if (!sourceId.HasValue)
        {
            return await GetGlobalResourceBindingLookupAsync(cancellationToken);
        }

        var entries = await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, sourceId.Value, cancellationToken);
        return Result.Success(ToResourceBindingLookup(entries));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetGlobalResourceBindingLookupAsync(CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(ResourceBindingScope.Global, null, cancellationToken);
        return Result.Success(ToResourceBindingLookup(entries));
    }

    private static IEnumerable<ResourceInfo> ToResourceBindingLookup(IEnumerable<ResourceBinding> entries)
        => entries
            .GroupBy(static entry => entry.Name, StringComparer.OrdinalIgnoreCase)
            .Select(static group => group.First())
            .OrderBy(static entry => entry.Name)
            .Select(static entry => new ResourceInfo(entry.Id, entry.Name));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetUserTeamLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Users.GetTeamsLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Teams.SearchAuthorizedAsync(userId, ResourceType.Team, PermissionLevel.Read, SpecificPermission.None, string.Empty, 50, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetUserRoleLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Roles.GetUserRoleLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Roles.GetAuthorizedAsync(userId, ResourceType.Role, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformDeploymentLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Platforms.GetDeploymentLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Deployments.GetAuthorizedInfoAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformStackLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Platforms.GetStackLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Stacks.GetAuthorizedInfoAsync(userId, ResourceType.Stack, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformRegistryLookupAsync(Guid? sourceId, Guid userId, CancellationToken cancellationToken)
        => sourceId.HasValue
            ? Result.Success(await unitOfWork.Platforms.GetRegistryLookupAsync(sourceId.Value, userId, cancellationToken))
            : Result.Success((await unitOfWork.Registries.GetAuthorizedAsync(userId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
                .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformImageLookupAsync(Guid? sourceId, CancellationToken cancellationToken)
    {
        if (!sourceId.HasValue)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("sourceResourceId is required for Platform -> Image lookup."));
        }

        var items = await unitOfWork.Images.GetByPlatformIdAsync(sourceId.Value, cancellationToken);
        return Result.Success(items.Select(static item => new ResourceInfo(item.Id, item.Name)));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformVolumeLookupAsync(Guid? sourceId, CancellationToken cancellationToken)
    {
        if (!sourceId.HasValue)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("sourceResourceId is required for Platform -> Volume lookup."));
        }

        if (!platformContainerCache.TryGetCacheEntry(sourceId.Value, out var platform, out var platformError))
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(platformError);
        }

        var volumeConnector = volumeConnectorFactory.GetConnector(platform.ConnectorType);
        var result = await volumeConnector.ListVolumesAsync(
            new ListdDockerVolumesCommand(
                PlatformAddress: platform.Address,
                Dangling: null,
                Driver: null,
                Name: null),
            cancellationToken);

        if (result.IsFailure(out var error, out var volumes))
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(error);
        }

        return Result.Success(volumes.Select(static item => new ResourceInfo(Guid.Empty, item.Name)));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformNetworkLookupAsync(Guid? sourceId, CancellationToken cancellationToken)
    {
        if (!sourceId.HasValue)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("sourceResourceId is required for Platform -> Network lookup."));
        }

        var result = await networkService.List(new ListNetworks(sourceId.Value), cancellationToken);
        if (result.IsFailure(out var error, out var networks))
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(error.Message);
        }

        return Result.Success(networks.Select(static item => new ResourceInfo(Guid.Empty, item.Name)));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertPlatformLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.Platforms.GetAuthorizedAsync(userId, ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertDeploymentLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.Deployments.GetAuthorizedInfoAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertStackLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.Stacks.GetAuthorizedInfoAsync(userId, ResourceType.Stack, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertGitRepositoryLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.GitRepositories.GetAuthorizedAsync(userId, ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertAutomationActionLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.AutomationActions.GetAuthorizedAsync(userId, ResourceType.AutomationAction, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertChannelLookupAsync(Guid userId, CancellationToken cancellationToken)
       => Result.Success((await unitOfWork.AlertRules.GetAuthorizedAlertChannelsAsync(userId, ResourceType.AlertChannel, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
               .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetImageRegistryLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Registries.GetAuthorizedAsync(userId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetTeamLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Teams.SearchAuthorizedAsync(userId, ResourceType.Team, PermissionLevel.Read, SpecificPermission.None, string.Empty, 50, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetUserLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Users.GetAuthorizedPagedAsync(userId, ResourceType.User, PermissionLevel.Read, SpecificPermission.None, 1, 50, null, cancellationToken))
            .Items
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetUserActorLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Users.GetAuthorizedPagedAsync(userId, ResourceType.User, PermissionLevel.Read, SpecificPermission.None, 1, 50, null, cancellationToken))
            .Items
            .Select(static item => new ResourceInfo(item.ActorId, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetPlatformLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Platforms.GetAuthorizedAsync(userId, ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAlertLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.AlertRules.GetAuthorizedAsync(userId, ResourceType.Alert, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetRoleLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Roles.GetAuthorizedAsync(userId, ResourceType.Role, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetRegistryLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Registries.GetAuthorizedAsync(userId, ResourceType.Registry, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetGitRepositoryLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.GitRepositories.GetAuthorizedAsync(userId, ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetOidcProviderLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.OidcProviders.GetAuthorizedAsync(userId, ResourceType.Binding, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.DisplayName)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetAutomationActionLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.AutomationActions.GetAuthorizedAsync(userId, ResourceType.AutomationAction, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetBackupRepositoryLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success(await unitOfWork.BackupRepositories.GetAuthorizedLookupAsync(userId, ResourceType.BackupRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetBackupPolicyLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.BackupPolicies.GetAuthorizedAsync(userId, ResourceType.BackupPolicy, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetBuildLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.BuildProjects.GetAuthorizedAsync(userId, ResourceType.Build, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetBuildAgentPoolLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.BuildAgentPools.GetAuthorizedAsync(userId, ResourceType.BuildAgentPool, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetDeploymentLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Deployments.GetAuthorizedInfoAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetStackLookupAsync(Guid userId, CancellationToken cancellationToken)
        => Result.Success((await unitOfWork.Stacks.GetAuthorizedInfoAsync(userId, ResourceType.Stack, PermissionLevel.Read, SpecificPermission.None, cancellationToken))
            .Select(static item => new ResourceInfo(item.Id, item.Name)));

    private async Task<Result<IEnumerable<ResourceInfo>>> GetImageLookupAsync(Guid userId, LookupContext context, CancellationToken cancellationToken)
    {
        if (context.PlatformId is null || context.PlatformId == Guid.Empty)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new BadRequestError("platformId is required for image lookup."));
        }

        var canAccessPlatform = await unitOfWork.Platforms.CanAccessAsync(userId, context.PlatformId.Value, cancellationToken);
        if (!canAccessPlatform)
        {
            return Result.Failure<IEnumerable<ResourceInfo>>(new NotFoundError("The scoped platform does not exist or is not accessible."));
        }

        var items = await unitOfWork.Images.GetByPlatformIdAsync(context.PlatformId.Value, cancellationToken);
        return Result.Success(items.Select(static item => new ResourceInfo(item.Id, item.Name)));
    }

    private async Task<Result<IEnumerable<ResourceInfo>>> GetLicenseLookupAsync(CancellationToken cancellationToken)
    {
        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
        return Result.Success<IEnumerable<ResourceInfo>>([new ResourceInfo(identity.InstanceId, "License")]);
    }

    private static string GetUnsupportedLookupMessage(LookupResourceType? sourceType, LookupResourceType targetType)
        => sourceType is null
            ? $"Lookup for target {targetType} is not supported."
            : $"Lookup from {sourceType.Value} to {targetType} is not supported.";

    private static Error GetSourceNotFoundError(LookupResourceType sourceType)
        => sourceType switch
        {
            LookupResourceType.Deployment => new NotFoundError("The source deployment does not exist or is not accessible."),
            LookupResourceType.Stack => new NotFoundError("The source stack does not exist or is not accessible."),
            LookupResourceType.User => new NotFoundError("The source user does not exist or is not accessible."),
            LookupResourceType.Platform => new NotFoundError("The source platform does not exist or is not accessible."),
            _ => new BadRequestError($"Source resource type {sourceType} is not supported for lookups.")
        };
}

public sealed record LookupContext(Guid? PlatformId = null);
