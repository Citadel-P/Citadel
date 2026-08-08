using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common;
using LightResults;

namespace Application.Features.SwarmServices;

internal static class SwarmServiceValidation
{
    public static async Task<bool> ExistsAndCanAccessAsync(
        Guid serviceId,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(serviceId, cancellationToken);
        return service is not null
            && await CanAccessPlatformAsync(service.PlatformId, unitOfWork, userContext, cancellationToken);
    }

    public static async Task<bool> CanAccessPlatformAsync(
        Guid platformId,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken) =>
        userContext.Current.IsAdmin
        || userContext.Current.ActorId == Constants.SystemId
        || await unitOfWork.Platforms.CanAccessAsync(
            userContext.Current.UserId,
            platformId,
            cancellationToken);

    public static async Task<Result<Platform>> ValidateAsync(
        SwarmServiceSpec spec,
        Guid platformId,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken,
        bool requireOnline = false)
    {
        if (spec.Image is null
            || spec.Command is null
            || spec.Arguments is null
            || spec.Environment is null
            || spec.Labels is null
            || spec.Ports is null
            || spec.NetworkIds is null
            || spec.Mounts is null
            || spec.Secrets is null
            || spec.Configs is null
            || spec.PlacementConstraints is null)
            return Result.Failure<Platform>(new BadRequestError("The Service configuration is incomplete."));

        if (spec.Ports.Any(static value => value is null)
            || spec.Mounts.Any(static value => value is null)
            || spec.Secrets.Any(static value => value is null)
            || spec.Configs.Any(static value => value is null)
            || spec.HealthCheck is { Test: null })
            return Result.Failure<Platform>(new BadRequestError("The Service configuration contains an invalid entry."));

        if (spec.Labels.Count > 100
            || spec.Labels.Any(static label => string.IsNullOrWhiteSpace(label.Key) || label.Value is null))
            return Result.Failure<Platform>(new BadRequestError(
                "Service labels require a non-empty key and are limited to 100 entries."));
        if (spec.Labels.Keys.Any(static key => key.StartsWith("com.citadel.", StringComparison.OrdinalIgnoreCase)))
            return Result.Failure<Platform>(new BadRequestError(
                "Service labels in the com.citadel namespace are reserved."));

        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null
            || !await CanAccessPlatformAsync(platformId, unitOfWork, userContext, cancellationToken))
            return Result.Failure<Platform>(new NotFoundError("The Docker Swarm platform does not exist or is not accessible."));

        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor)
            return Result.Failure<Platform>(new BadRequestError("Managed Swarm Services require a Docker Swarm platform."));

        if (requireOnline && (platform.Status != PlatformStatus.Online || !descriptor.ControlAvailable))
            return Result.Failure<Platform>(new ConflictError("The Docker Swarm manager is not available."));

        if (spec.SchedulingMode == SwarmServiceSchedulingMode.Replicated && spec.Replicas is null)
            return Result.Failure<Platform>(new BadRequestError("Replica count is required for a replicated Service."));
        if (spec.SchedulingMode == SwarmServiceSchedulingMode.Global && spec.Replicas is not null)
            return Result.Failure<Platform>(new BadRequestError("Replica count is not valid for a global Service."));
        if (spec.Replicas < 0)
            return Result.Failure<Platform>(new BadRequestError("Replica count cannot be negative."));
        if (spec.UpdateBehavior != UpdateBehavior.Disabled
            && spec.Image is not SwarmExternalImage)
            return Result.Failure<Platform>(new BadRequestError(
                "Image update checks require an external tagged image."));
        if (spec.UpdateBehavior != UpdateBehavior.Disabled
            && spec.Image is SwarmExternalImage { ImageTag: var imageTag }
            && imageTag.Contains('@', StringComparison.Ordinal))
            return Result.Failure<Platform>(new BadRequestError(
                "Image update checks are not available for a digest-pinned image."));

        if (WebhookConfigurationValidation.GetAuthenticationError(spec.Webhook) is { } webhookError)
            return Result.Failure<Platform>(new BadRequestError(webhookError));
        if (spec.Webhook?.Secret?.Length > 256)
            return Result.Failure<Platform>(new BadRequestError(
                "Service webhook secret cannot exceed 256 characters."));
        if (spec.Webhook?.BranchFilter?.Length > 256)
            return Result.Failure<Platform>(new BadRequestError(
                "Service webhook branch filter cannot exceed 256 characters."));
        if (spec.Webhook is { Enabled: true } && spec.UpdateBehavior == UpdateBehavior.Disabled)
            return Result.Failure<Platform>(new BadRequestError(
                "Service update webhooks require Notify or Auto deploy update behavior."));
        if (spec.Webhook is { Enabled: true } && spec.Image is not SwarmExternalImage)
            return Result.Failure<Platform>(new BadRequestError(
                "Service update webhooks require an external tagged image."));
        if (spec.Webhook is { Enabled: true }
            && spec.Image is SwarmExternalImage { ImageTag: var webhookImageTag }
            && webhookImageTag.Contains('@', StringComparison.Ordinal))
            return Result.Failure<Platform>(new BadRequestError(
                "Service update webhooks are not available for a digest-pinned image."));

        var invalidPort = spec.Ports.FirstOrDefault(port =>
            port.TargetPort is < 1 or > 65535
            || port.PublishedPort is < 1 or > 65535
            || (!string.Equals(port.Protocol, "tcp", StringComparison.OrdinalIgnoreCase)
                && !string.Equals(port.Protocol, "udp", StringComparison.OrdinalIgnoreCase)
                && !string.Equals(port.Protocol, "sctp", StringComparison.OrdinalIgnoreCase)));
        if (invalidPort is not null)
            return Result.Failure<Platform>(new BadRequestError("Service ports must use a valid port number and TCP, UDP, or SCTP protocol."));

        var duplicatePort = spec.Ports
            .Where(static port => port.PublishedPort is not null)
            .GroupBy(static port => (port.PublishedPort, Protocol: port.Protocol.ToLowerInvariant(), port.PublishMode))
            .Any(static group => group.Count() > 1);
        if (duplicatePort)
            return Result.Failure<Platform>(new BadRequestError("A published Service port can only be configured once per protocol and publish mode."));

        if (spec.NetworkIds.Count != spec.NetworkIds.Distinct(StringComparer.Ordinal).Count())
            return Result.Failure<Platform>(new BadRequestError("A Service network can only be selected once."));

        if (spec.Mounts.Any(static mount =>
                string.IsNullOrWhiteSpace(mount.Source)
                || string.IsNullOrWhiteSpace(mount.Target)
                || !mount.Target.StartsWith("/", StringComparison.Ordinal)))
            return Result.Failure<Platform>(new BadRequestError("Every Service mount requires a source and an absolute container target path."));
        if (spec.Mounts.GroupBy(static mount => mount.Target, StringComparer.Ordinal).Any(static group => group.Count() > 1))
            return Result.Failure<Platform>(new BadRequestError("A container mount target can only be configured once."));

        if (spec.Secrets.Any(static secret =>
                string.IsNullOrWhiteSpace(secret.SecretId)
                || string.IsNullOrWhiteSpace(secret.SecretName)
                || string.IsNullOrWhiteSpace(secret.TargetName))
            || spec.Secrets.Select(static secret => secret.SecretId).Distinct(StringComparer.Ordinal).Count() != spec.Secrets.Count
            || spec.Secrets.Select(static secret => secret.TargetName).Distinct(StringComparer.Ordinal).Count() != spec.Secrets.Count)
            return Result.Failure<Platform>(new BadRequestError("Every Swarm Secret requires a unique Secret and target name."));

        if (spec.Configs.Any(static config =>
                string.IsNullOrWhiteSpace(config.ConfigId)
                || string.IsNullOrWhiteSpace(config.ConfigName)
                || string.IsNullOrWhiteSpace(config.TargetName))
            || spec.Configs.Select(static config => config.ConfigId).Distinct(StringComparer.Ordinal).Count() != spec.Configs.Count
            || spec.Configs.Select(static config => config.TargetName).Distinct(StringComparer.Ordinal).Count() != spec.Configs.Count)
            return Result.Failure<Platform>(new BadRequestError("Every Swarm Config requires a unique Config and target name."));

        if (spec.HealthCheck is { } healthCheck
            && (healthCheck.Test.Count == 0
                || healthCheck.Test.Any(string.IsNullOrWhiteSpace)
                || IsNegative(healthCheck.IntervalNanoseconds)
                || IsNegative(healthCheck.TimeoutNanoseconds)
                || IsNegative(healthCheck.Retries)
                || IsNegative(healthCheck.StartPeriodNanoseconds)))
            return Result.Failure<Platform>(new BadRequestError("The Service health check requires a command and non-negative timing values."));
        if (IsNegative(spec.StopGracePeriodNanoseconds))
            return Result.Failure<Platform>(new BadRequestError("The stop grace period cannot be negative."));

        if (spec.Resources is { } resources
            && (IsNegative(resources.LimitNanoCpus)
                || IsNegative(resources.LimitMemoryBytes)
                || IsNegative(resources.ReservationNanoCpus)
                || IsNegative(resources.ReservationMemoryBytes)
                || Exceeds(resources.ReservationNanoCpus, resources.LimitNanoCpus)
                || Exceeds(resources.ReservationMemoryBytes, resources.LimitMemoryBytes)))
            return Result.Failure<Platform>(new BadRequestError("Resource reservations and limits must be non-negative, and a reservation cannot exceed its limit."));

        if (spec.RestartPolicy is { } restart
            && (IsNegative(restart.DelayNanoseconds)
                || IsNegative(restart.MaximumAttempts)
                || IsNegative(restart.WindowNanoseconds)))
            return Result.Failure<Platform>(new BadRequestError("Restart policy values cannot be negative."));
        if (spec.UpdatePolicy is { } update
            && (update.Parallelism < 0 || IsNegative(update.DelayNanoseconds)))
            return Result.Failure<Platform>(new BadRequestError("Rolling-update values cannot be negative."));

        if (spec.Command.Count > 100
            || spec.Arguments.Count > 200
            || spec.Environment.Count > 500
            || spec.Ports.Count > 100
            || spec.NetworkIds.Count > 100
            || spec.Mounts.Count > 100
            || spec.Secrets.Count > 100
            || spec.Configs.Count > 100
            || spec.PlacementConstraints.Count > 100)
            return Result.Failure<Platform>(new BadRequestError("The Service configuration contains too many entries."));

        if (spec.NetworkIds.Count != 0)
        {
            var networks = await unitOfWork.Swarm.GetNetworksAsync(platformId, cancellationToken);
            var ingressNetworkIds = networks
                .Where(static network => network.IsIngress)
                .Select(static network => network.DockerNetworkId)
                .ToHashSet(StringComparer.Ordinal);
            if (spec.NetworkIds.Any(ingressNetworkIds.Contains))
                return Result.Failure<Platform>(new BadRequestError(
                    "The Swarm ingress network is managed by Docker and cannot be attached to a Service explicitly."));

            var networkIds = networks
                .Where(static network =>
                    !network.IsIngress
                    && string.Equals(network.Scope, "swarm", StringComparison.OrdinalIgnoreCase))
                .Select(static network => network.DockerNetworkId)
                .ToHashSet(StringComparer.Ordinal);
            if (spec.NetworkIds.Any(id => !networkIds.Contains(id)))
                return Result.Failure<Platform>(new BadRequestError("One or more selected overlay Networks do not exist on this Swarm."));
        }

        if (spec.Secrets.Count != 0)
        {
            var secrets = (await unitOfWork.Swarm.GetSecretsAsync(platformId, cancellationToken))
                .ToDictionary(static secret => secret.DockerSecretId, static secret => secret.Name, StringComparer.Ordinal);
            if (spec.Secrets.Any(secret =>
                    !secrets.TryGetValue(secret.SecretId, out var name)
                    || !string.Equals(name, secret.SecretName, StringComparison.Ordinal)))
                return Result.Failure<Platform>(new BadRequestError(
                    "One or more selected Docker Swarm Secrets do not exist or no longer match this Swarm."));
        }

        if (spec.Configs.Count != 0)
        {
            var configs = (await unitOfWork.Swarm.GetConfigsAsync(platformId, cancellationToken))
                .ToDictionary(static config => config.DockerConfigId, static config => config.Name, StringComparer.Ordinal);
            if (spec.Configs.Any(config =>
                    !configs.TryGetValue(config.ConfigId, out var name)
                    || !string.Equals(name, config.ConfigName, StringComparison.Ordinal)))
                return Result.Failure<Platform>(new BadRequestError(
                    "One or more selected Docker Swarm Configs do not exist or no longer match this Swarm."));
        }

        var imageValidation = await ValidateImageAsync(spec.Image, unitOfWork, userContext, cancellationToken);
        return imageValidation.IsFailure(out var imageError)
            ? Result.Failure<Platform>(imageError)
            : Result.Success(platform);
    }

    private static async Task<Result> ValidateImageAsync(
        SwarmServiceImageInfo image,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        switch (image)
        {
            case SwarmExternalImage external when external.RegistryId == Guid.Empty || string.IsNullOrWhiteSpace(external.ImageTag):
                return Result.Failure(new BadRequestError("An external Registry and tagged image are required."));
            case SwarmExternalImage external when await unitOfWork.Registries.GetAsync(external.RegistryId, cancellationToken) is null:
                return Result.Failure(new NotFoundError("The selected Registry does not exist."));
            case SwarmExternalImage external when !userContext.Current.IsAdmin
                && !(await unitOfWork.Registries.GetAuthorizedAsync(
                    userContext.Current.UserId,
                    ResourceType.Registry,
                    PermissionLevel.Read,
                    SpecificPermission.None,
                    cancellationToken)).Any(registry => registry.Id == external.RegistryId):
                return Result.Failure(new NotFoundError("The selected Registry does not exist or is not accessible."));
            case SwarmBuildImage build when build.BuildProjectId == Guid.Empty:
                return Result.Failure(new BadRequestError("A build project is required."));
            case SwarmBuildImage build when await unitOfWork.BuildProjects.GetAsync(build.BuildProjectId, cancellationToken, includeArchived: true) is null:
                return Result.Failure(new NotFoundError("The selected build project does not exist."));
            case SwarmBuildImage build when !userContext.Current.IsAdmin
                && !await unitOfWork.BuildProjects.CanAccessAsync(
                    userContext.Current.UserId,
                    build.BuildProjectId,
                    ResourceType.Build,
                    PermissionLevel.Read,
                    SpecificPermission.None,
                    cancellationToken):
                return Result.Failure(new NotFoundError("The selected build project does not exist or is not accessible."));
            default:
                return Result.Success();
        }
    }

    private static bool IsNegative(long? value) => value < 0;
    private static bool IsNegative(int? value) => value < 0;
    private static bool Exceeds(long? reservation, long? limit) =>
        reservation is not null && limit is not null && reservation > limit;
}
