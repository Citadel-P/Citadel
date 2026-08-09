using Application.Services;
using Application.Features.Containers.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using System.Text;

namespace Application.Features.Stacks.Queries;

internal sealed record SwarmStackImportContext(
    Platform Platform,
    string Namespace,
    IReadOnlyList<SwarmServiceResult> Services,
    IReadOnlyList<SwarmNetworkResult> Networks,
    IReadOnlyList<SwarmSecretResult> Secrets,
    IReadOnlyList<SwarmConfigResult> Configs,
    Guid? OrphanedOwnerStackId = null);

internal static class SwarmStackImportDraftFactory
{
    internal static async Task<Result<SwarmStackImportContext>> LoadContextAsync(
        Guid platformId,
        string stackNamespace,
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        IPermissionService permissionService,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(stackNamespace))
            return Result.Failure<SwarmStackImportContext>(new BadRequestError("Docker Stack namespace is required."));
        if (stackNamespace.Length > 63)
            return Result.Failure<SwarmStackImportContext>(new BadRequestError("Docker Stack namespace cannot exceed 63 characters."));

        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null || platform.PlatformDescriptor.Type != PlatformType.DockerSwarm)
            return Result.Failure<SwarmStackImportContext>(new NotFoundError("Swarm platform does not exist."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmStackImportContext>(new ConflictError("Platform is offline."));

        var user = userContext.Current;
        if (!user.IsAdmin)
        {
            if (!await unitOfWork.Platforms.CanAccessAsync(user.UserId, platformId, cancellationToken))
                return Result.Failure<SwarmStackImportContext>(new NotFoundError("Swarm platform does not exist."));

            var permissions = await permissionService.ResolvePermissionsAsync(
                user.UserId,
                ResourceType.Platform,
                platformId,
                cancellationToken);
            if (!permissions.Has(PermissionLevel.Read, SpecificPermission.Inspect))
            {
                return Result.Failure<SwarmStackImportContext>(
                    new ForbiddenError("Missing specific permission [Inspect] on [Platform]."));
            }
        }

        var projections = await unitOfWork.Swarm.GetServicesAsync(platformId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var servicesTask = connector.ListServicesAsync(
            new ListSwarmServicesCommand(platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var networksTask = connector.ListNetworksAsync(
            new ListSwarmNetworksCommand(platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var secretsTask = connector.ListSecretsAsync(
            new ListSwarmSecretsCommand(platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        var configsTask = connector.ListConfigsAsync(
            new ListSwarmConfigsCommand(platform.Address, SwarmInventoryLimits.AuthoritativeSnapshotItems),
            cancellationToken);
        await Task.WhenAll(servicesTask, networksTask, secretsTask, configsTask);

        var servicesResult = await servicesTask;
        if (servicesResult.IsFailure(out var serviceError, out var allServices))
            return Result.Failure<SwarmStackImportContext>(new BadRequestError(serviceError.Message));
        var networksResult = await networksTask;
        if (networksResult.IsFailure(out var networkError, out var allNetworks))
            return Result.Failure<SwarmStackImportContext>(new BadRequestError(networkError.Message));
        var secretsResult = await secretsTask;
        if (secretsResult.IsFailure(out var secretError, out var allSecrets))
            return Result.Failure<SwarmStackImportContext>(new BadRequestError(secretError.Message));
        var configsResult = await configsTask;
        if (configsResult.IsFailure(out var configError, out var allConfigs))
            return Result.Failure<SwarmStackImportContext>(new BadRequestError(configError.Message));

        var services = allServices
            .Where(service => HasNamespace(service.Labels, stackNamespace))
            .OrderBy(static service => service.Id, StringComparer.Ordinal)
            .ToArray();
        if (services.Length == 0)
            return Result.Failure<SwarmStackImportContext>(new NotFoundError($"Docker Stack '{stackNamespace}' has no Services."));

        var projected = projections
            .Where(service => string.Equals(service.DockerStackNamespace, stackNamespace, StringComparison.Ordinal))
            .ToDictionary(static service => service.DockerServiceId, StringComparer.Ordinal);
        if (services.Any(service =>
                projected.TryGetValue(service.Id, out var projection)
                && (projection.StackId is not null || projection.Ownership != SwarmServiceOwnership.DockerStackExternal)))
        {
            return Result.Failure<SwarmStackImportContext>(
                new ConflictError("One or more Docker Stack Services are already managed by Citadel."));
        }

        if (projected.Count != services.Length || services.Any(service => !projected.ContainsKey(service.Id)))
        {
            return Result.Failure<SwarmStackImportContext>(
                new ConflictError("Docker Stack Services have not finished synchronizing. Refresh and try again."));
        }

        Guid? orphanedOwnerId = null;
        var citadelOwnedServices = services.Where(service => HasCitadelLabel(service.Labels)).ToArray();
        if (citadelOwnedServices.Length > 0)
        {
            if (citadelOwnedServices.Length != services.Length)
            {
                return Result.Failure<SwarmStackImportContext>(
                    new ConflictError("Docker Stack contains a mix of Citadel-owned and unmanaged Services."));
            }

            foreach (var service in citadelOwnedServices)
            {
                if (!TryGetOrphanedStackOwnerId(service.Labels, out var ownerId))
                {
                    return Result.Failure<SwarmStackImportContext>(
                        new ConflictError($"Service '{service.Name}' has invalid or conflicting Citadel ownership labels."));
                }

                if (orphanedOwnerId is not null && orphanedOwnerId != ownerId)
                {
                    return Result.Failure<SwarmStackImportContext>(
                        new ConflictError("Docker Stack Services reference different Citadel owners."));
                }

                orphanedOwnerId = ownerId;
            }

            if (orphanedOwnerId is not null
                && await unitOfWork.Stacks.ExistsAsync(orphanedOwnerId.Value, cancellationToken))
            {
                return Result.Failure<SwarmStackImportContext>(
                    new ConflictError("Docker Stack is owned by an existing Citadel Stack."));
            }
        }

        return new SwarmStackImportContext(
            platform,
            stackNamespace,
            services,
            allNetworks.Where(network => HasNamespace(network.Labels, stackNamespace)).OrderBy(static value => value.Id, StringComparer.Ordinal).ToArray(),
            allSecrets.Where(secret => HasNamespace(secret.Labels, stackNamespace)).OrderBy(static value => value.Id, StringComparer.Ordinal).ToArray(),
            allConfigs.Where(config => HasNamespace(config.Labels, stackNamespace)).OrderBy(static value => value.Id, StringComparer.Ordinal).ToArray(),
            orphanedOwnerId);
    }

    internal static ComposeProjectImportDraft Create(
        SwarmStackImportContext context,
        string name,
        IAdoptionFingerprintService fingerprintService)
        => new(
            StackImportKind.SwarmStack,
            new ComposeProjectImportSource(
                context.Platform.Id,
                context.Platform.Name,
                context.Namespace,
                context.Services.Select(static service => service.Id).ToArray(),
                context.Services.Select(static service => service.Name).ToArray(),
                GetRuntimeServices(context)),
            new ComposeProjectStackDraft(
                name,
                context.Platform.Id,
                $"Imported from Docker Swarm Stack {context.Namespace}.",
                StackDriftPolicy.Disabled,
                []),
            [],
            ComputeRuntimeFingerprint(context, fingerprintService));

    internal static ComposeProjectImportValidation CreateValidation(
        SwarmStackImportContext context,
        ComposeProjectSourceAnalysis source,
        IAdoptionFingerprintService fingerprintService)
    {
        var runtimeServices = GetRuntimeServices(context);
        var issues = new List<AdoptionIssue>();
        ComposeProjectImportDraftFactory.AddSwarmCompatibilityIssues(source, issues);
        var comparisons = runtimeServices.Select(runtime =>
        {
            source.Services.TryGetValue(runtime.Name, out var desired);
            return new ComposeProjectServiceComparison(
                runtime.Name,
                runtime.ContainerCount,
                runtime.Image,
                desired is not null,
                desired?.Image);
        }).ToList();

        foreach (var desired in source.Services.Values.Where(desired =>
                     !runtimeServices.Any(runtime => string.Equals(runtime.Name, desired.ServiceName, StringComparison.OrdinalIgnoreCase))))
        {
            comparisons.Add(new ComposeProjectServiceComparison(desired.ServiceName, 0, null, true, desired.Image));
            issues.Add(new AdoptionIssue(
                "SOURCE_SERVICE_NOT_RUNNING",
                $"Source Service '{desired.ServiceName}' is not present in the Docker Stack. A future Apply may create it.",
                AdoptionIssueSeverity.Warning));
        }

        if (!comparisons.Any(comparison => comparison.DefinedInSource && comparison.RuntimeContainerCount > 0))
        {
            issues.Add(new AdoptionIssue(
                "NO_MATCHING_SERVICES",
                "The selected source does not define any Service in the Docker Stack.",
                AdoptionIssueSeverity.Blocker));
        }

        foreach (var comparison in comparisons.Where(comparison => comparison.RuntimeContainerCount > 0 && !comparison.DefinedInSource))
        {
            issues.Add(new AdoptionIssue(
                "RUNTIME_SERVICE_NOT_IN_SOURCE",
                $"Runtime Service '{comparison.Name}' is not defined by the selected source. A future Apply may remove it.",
                AdoptionIssueSeverity.Warning));
        }

        foreach (var comparison in comparisons.Where(comparison =>
                     comparison.RuntimeContainerCount > 0
                     && comparison.DefinedInSource
                     && !string.IsNullOrWhiteSpace(comparison.RuntimeImage)
                     && !string.IsNullOrWhiteSpace(comparison.SourceImage)
                     && !string.Equals(comparison.RuntimeImage, comparison.SourceImage, StringComparison.OrdinalIgnoreCase)))
        {
            issues.Add(new AdoptionIssue(
                "SERVICE_IMAGE_DIFFERS",
                $"Service '{comparison.Name}' currently uses '{comparison.RuntimeImage}', while the selected source defines '{comparison.SourceImage}'.",
                AdoptionIssueSeverity.Warning));
        }

        return new ComposeProjectImportValidation(
            comparisons.OrderBy(static comparison => comparison.Name, StringComparer.OrdinalIgnoreCase).ToArray(),
            issues,
            fingerprintService.Sign($"{ComputeRuntimeFingerprint(context, fingerprintService)}|{source.SourceDigest}"),
            [],
            false);
    }

    internal static string ComputeRuntimeFingerprint(
        SwarmStackImportContext context,
        IAdoptionFingerprintService fingerprintService)
    {
        var builder = new StringBuilder();
        builder.Append(context.Platform.Id).Append('|').Append(context.Namespace).Append('|');
        foreach (var service in context.Services)
        {
            builder.Append(service.Id).Append('|').Append(service.VersionIndex).Append('|')
                .Append(service.RuntimeHash).Append('|').Append(service.Name).Append('|');
        }
        foreach (var resourceId in context.Networks.Select(static value => value.Id)
                     .Concat(context.Secrets.Select(static value => value.Id))
                     .Concat(context.Configs.Select(static value => value.Id)))
        {
            builder.Append(resourceId).Append('|');
        }

        return fingerprintService.Sign(builder.ToString());
    }

    private static IReadOnlyList<ComposeProjectRuntimeService> GetRuntimeServices(SwarmStackImportContext context)
        => context.Services.Select(service => new ComposeProjectRuntimeService(
                service.Name.StartsWith($"{context.Namespace}_", StringComparison.Ordinal)
                    ? service.Name[(context.Namespace.Length + 1)..]
                    : service.Name,
                service.Image,
                service.DesiredTaskCount,
                service.RunningTaskCount >= service.DesiredTaskCount
                    ? [ContainerStateStatus.Running]
                    : [ContainerStateStatus.Exited]))
            .OrderBy(static service => service.Name, StringComparer.OrdinalIgnoreCase)
            .ToArray();

    private static bool HasNamespace(IReadOnlyDictionary<string, string> labels, string stackNamespace)
        => labels.TryGetValue("com.docker.stack.namespace", out var value)
           && string.Equals(value, stackNamespace, StringComparison.Ordinal);

    private static bool HasCitadelLabel(IReadOnlyDictionary<string, string> labels)
        => labels.Keys.Any(static key => key.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase));

    private static bool TryGetOrphanedStackOwnerId(
        IReadOnlyDictionary<string, string> labels,
        out Guid ownerId)
    {
        ownerId = Guid.Empty;
        return labels.TryGetValue(CitadelLabels.Managed, out var managed)
               && string.Equals(managed, "true", StringComparison.OrdinalIgnoreCase)
               && labels.TryGetValue(CitadelLabels.StackId, out var value)
               && Guid.TryParse(value, out ownerId)
               && !labels.ContainsKey("com.citadel.service-id")
               && !labels.ContainsKey("com.citadel.deployment-id");
    }
}
