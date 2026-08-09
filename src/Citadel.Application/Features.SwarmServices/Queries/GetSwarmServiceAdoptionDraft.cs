using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Text;

namespace Application.Features.SwarmServices.Queries;

public sealed record SwarmServiceAdoptionSource(
    string DockerServiceId,
    string Name,
    Guid PlatformId,
    string PlatformName);

public sealed record SwarmServiceAdoptionIssue(string Code, string Message);

public sealed record SwarmServiceAdoptionDraft(
    SwarmServiceAdoptionSource Source,
    string Name,
    string? Description,
    SwarmServiceSpec Spec,
    IReadOnlyList<SwarmServiceAdoptionIssue> Issues,
    string PreviewFingerprint);

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write)]
[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect,
    ResourceIdProperty = nameof(PlatformId))]
public sealed record GetSwarmServiceAdoptionDraft(Guid PlatformId, string DockerServiceId)
    : IQuery<Result<SwarmServiceAdoptionDraft>>;

internal sealed class GetSwarmServiceAdoptionDraftHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IAdoptionFingerprintService fingerprintService)
    : IQueryHandler<GetSwarmServiceAdoptionDraft, Result<SwarmServiceAdoptionDraft>>
{
    public async ValueTask<Result<SwarmServiceAdoptionDraft>> Handle(
        GetSwarmServiceAdoptionDraft query,
        CancellationToken cancellationToken)
    {
        var context = await SwarmServiceAdoptionDraftFactory.LoadAsync(
            query.PlatformId,
            query.DockerServiceId,
            unitOfWork,
            connectorFactory,
            cancellationToken);
        if (!context.IsSuccess(out var value))
            return Result.Failure<SwarmServiceAdoptionDraft>(context.Errors);

        var name = await GetAvailableNameAsync(value.Service.Name, query.PlatformId, cancellationToken);
        return Result.Success(SwarmServiceAdoptionDraftFactory.Create(value, name, fingerprintService));
    }

    private async Task<string> GetAvailableNameAsync(
        string source,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var baseName = NormalizeName(source);
        if (!await unitOfWork.SwarmServices.ExistsAsync(platformId, baseName, cancellationToken))
            return baseName;

        for (var suffix = 2; suffix <= 100; suffix++)
        {
            var suffixText = $"-{suffix}";
            var candidate = $"{baseName[..Math.Min(baseName.Length, 64 - suffixText.Length)].TrimEnd('-', '_')}{suffixText}";
            if (!await unitOfWork.SwarmServices.ExistsAsync(platformId, candidate, cancellationToken))
                return candidate;
        }

        return $"{baseName[..Math.Min(baseName.Length, 55)].TrimEnd('-', '_')}-{Guid.NewGuid():N}"[..64];
    }

    private static string NormalizeName(string value)
    {
        var builder = new StringBuilder(Math.Min(value.Length, 64));
        foreach (var character in value.Trim())
        {
            if (builder.Length == 64)
                break;
            builder.Append(char.IsAsciiLetterOrDigit(character) || character is '-' or '_'
                ? character
                : '-');
        }
        var normalized = builder.ToString().Trim('-', '_');
        return normalized.Length == 0 ? "adopted-service" : normalized;
    }
}

internal sealed record SwarmServiceAdoptionContext(
    Platform Platform,
    SwarmServiceProjection Projection,
    SwarmServiceResult Service);

internal static class SwarmServiceAdoptionDraftFactory
{
    internal static async Task<Result<SwarmServiceAdoptionContext>> LoadAsync(
        Guid platformId,
        string dockerServiceId,
        IUnitOfWork unitOfWork,
        IConnectorFactory<ISwarmConnector> connectorFactory,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmServiceAdoptionContext>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmServiceAdoptionContext>(new BadRequestError("Service adoption is only available on Docker Swarm platforms."));
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<SwarmServiceAdoptionContext>(new ConflictError("Platform is offline."));

        var projection = await unitOfWork.Swarm.GetServiceAsync(platformId, dockerServiceId, cancellationToken);
        if (projection is null)
            return Result.Failure<SwarmServiceAdoptionContext>(new NotFoundError("Swarm Service does not exist."));
        if (projection.IsStale)
            return Result.Failure<SwarmServiceAdoptionContext>(new ConflictError("Swarm Service inventory is stale. Refresh the platform and try again."));
        if (projection.Ownership != SwarmServiceOwnership.Unmanaged)
        {
            var message = projection.Ownership == SwarmServiceOwnership.DockerStackExternal
                ? "Services owned by a Docker stack must be imported with their stack."
                : "Swarm Service is already managed by Citadel or has conflicting ownership labels.";
            return Result.Failure<SwarmServiceAdoptionContext>(new ConflictError(message));
        }

        if (await GetOwnershipConflictAsync(projection.Labels, unitOfWork, cancellationToken) is { } projectionConflict)
            return Result.Failure<SwarmServiceAdoptionContext>(projectionConflict);

        if (await unitOfWork.SwarmServices.GetByDockerServiceIdAsync(platformId, dockerServiceId, cancellationToken) is not null)
            return Result.Failure<SwarmServiceAdoptionContext>(new ConflictError("Swarm Service is already managed by Citadel."));

        var result = await connectorFactory.GetConnector(platform.ConnectorType).InspectServiceAsync(
            new InspectSwarmServiceCommand(platform.Address, dockerServiceId),
            cancellationToken);
        if (!result.IsSuccess(out var service, out var error))
            return Result.Failure<SwarmServiceAdoptionContext>(error!);
        if (!string.Equals(service.Id, dockerServiceId, StringComparison.Ordinal))
            return Result.Failure<SwarmServiceAdoptionContext>(new ConflictError("Docker returned a different Service. Refresh and try again."));
        if (service.Definition is null)
            return Result.Failure<SwarmServiceAdoptionContext>(new BadRequestError("Docker did not return a Service definition that Citadel can adopt."));
        if (service.Name.Length > 63)
            return Result.Failure<SwarmServiceAdoptionContext>(new BadRequestError("Docker Service name is longer than Citadel can manage."));
        if (service.Labels.TryGetValue("com.docker.stack.namespace", out var stackNamespace)
            && !string.IsNullOrWhiteSpace(stackNamespace))
        {
            return Result.Failure<SwarmServiceAdoptionContext>(
                new ConflictError("Services owned by a Docker stack must be imported with their stack."));
        }
        if (await GetOwnershipConflictAsync(service.Labels, unitOfWork, cancellationToken) is { } inspectedConflict)
            return Result.Failure<SwarmServiceAdoptionContext>(inspectedConflict);

        return Result.Success(new SwarmServiceAdoptionContext(platform, projection, service));
    }

    internal static SwarmServiceAdoptionDraft Create(
        SwarmServiceAdoptionContext context,
        string name,
        IAdoptionFingerprintService fingerprintService)
    {
        var issues = (context.Service.AdoptionWarnings ?? [])
            .Select((message, index) => new SwarmServiceAdoptionIssue($"unsupported-{index + 1}", message))
            .ToList();
        var sensitiveNames = GetSensitiveEnvironmentNames(context.Service.Definition!.Environment);
        foreach (var sensitiveName in sensitiveNames)
        {
            issues.Add(new SwarmServiceAdoptionIssue(
                $"sensitive-environment-{issues.Count + 1}",
                $"Enter a new value or Citadel binding for sensitive environment variable '{sensitiveName}'."));
        }
        var spec = context.Service.Definition with
        {
            Environment = RedactSensitiveEnvironment(context.Service.Definition.Environment),
            Labels = context.Service.Definition.Labels
                .Where(static label => !label.Key.StartsWith("com.citadel.", StringComparison.OrdinalIgnoreCase))
                .ToDictionary(static label => label.Key, static label => label.Value, StringComparer.Ordinal)
        };
        return new SwarmServiceAdoptionDraft(
            new SwarmServiceAdoptionSource(
                context.Service.Id,
                context.Service.Name,
                context.Platform.Id,
                context.Platform.Name),
            name,
            $"Adopted from Docker Swarm Service {context.Service.Name}.",
            spec,
            issues,
            ComputeFingerprint(context, fingerprintService));
    }

    internal static string? GetRedactedSensitiveEnvironmentName(IReadOnlyList<string> environment)
    {
        foreach (var entry in environment)
        {
            var separator = entry.IndexOf('=');
            if (separator <= 0)
                continue;

            var name = entry[..separator].Trim();
            if (ContainerInspectionRedactor.IsSensitiveEnvironmentName(name)
                && string.Equals(
                    entry[(separator + 1)..],
                    ContainerInspectionRedactor.RedactedValue,
                    StringComparison.Ordinal))
                return name;
        }
        return null;
    }

    private static IReadOnlyList<string> RedactSensitiveEnvironment(IReadOnlyList<string> environment)
        => environment.Select(entry =>
        {
            var separator = entry.IndexOf('=');
            if (separator <= 0 || separator == entry.Length - 1)
                return entry;

            var name = entry[..separator].Trim();
            return ContainerInspectionRedactor.IsSensitiveEnvironmentName(name)
                ? $"{name}={ContainerInspectionRedactor.RedactedValue}"
                : entry;
        }).ToArray();

    private static IReadOnlyList<string> GetSensitiveEnvironmentNames(IReadOnlyList<string> environment)
        => environment
            .Select(static entry => entry.Split('=', 2)[0].Trim())
            .Where(ContainerInspectionRedactor.IsSensitiveEnvironmentName)
            .Distinct(StringComparer.Ordinal)
            .ToArray();

    private static bool HasCitadelLabel(IReadOnlyDictionary<string, string> labels)
        => labels.Keys.Any(static key => key.StartsWith("com.citadel.", StringComparison.OrdinalIgnoreCase));

    private static async Task<ConflictError?> GetOwnershipConflictAsync(
        IReadOnlyDictionary<string, string> labels,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (!HasCitadelLabel(labels))
            return null;
        if (!TryGetOrphanedServiceOwnerId(labels, out var ownerId))
            return new ConflictError("Swarm Service has invalid or conflicting Citadel ownership labels.");
        return await unitOfWork.SwarmServices.GetAsync(ownerId, cancellationToken) is null
            ? null
            : new ConflictError("Swarm Service is owned by an existing Citadel Service.");
    }

    private static bool TryGetOrphanedServiceOwnerId(
        IReadOnlyDictionary<string, string> labels,
        out Guid ownerId)
    {
        ownerId = Guid.Empty;
        return labels.TryGetValue("com.citadel.managed", out var managed)
               && string.Equals(managed, "true", StringComparison.OrdinalIgnoreCase)
               && labels.TryGetValue("com.citadel.service-id", out var value)
               && Guid.TryParse(value, out ownerId)
               && !labels.ContainsKey("com.citadel.stack-id")
               && !labels.ContainsKey("com.citadel.deployment-id");
    }

    internal static string ComputeFingerprint(
        SwarmServiceAdoptionContext context,
        IAdoptionFingerprintService fingerprintService)
    {
        var value = new StringBuilder(1024);
        value.Append(context.Platform.Id).Append('|')
            .Append(context.Service.Id).Append('|')
            .Append(context.Service.VersionIndex).Append('|')
            .Append(context.Service.RuntimeHash).Append('|')
            .Append(context.Service.Name).Append('|')
            .Append(context.Service.Image);
        foreach (var (key, labelValue) in context.Service.Labels.OrderBy(static item => item.Key, StringComparer.Ordinal))
            value.Append('|').Append(key).Append('=').Append(labelValue);
        return fingerprintService.Sign(value.ToString());
    }
}
