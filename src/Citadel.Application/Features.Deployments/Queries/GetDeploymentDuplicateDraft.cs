using Application.Features;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentDuplicateDraft(Guid Id) : IQuery<Result<DeploymentDuplicateDraft>>;

public sealed record DeploymentDuplicateDraft(
    string Name,
    string SourceName,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    IReadOnlyCollection<Guid> TagIds,
    IReadOnlyCollection<DuplicateDraftWarning> Warnings);

internal sealed class GetDeploymentDuplicateDraftHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetDeploymentDuplicateDraft, Result<DeploymentDuplicateDraft>>
{
    public async ValueTask<Result<DeploymentDuplicateDraft>> Handle(GetDeploymentDuplicateDraft query, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken);
        if (deployment is null)
            return Result.Failure<DeploymentDuplicateDraft>(new NotFoundError($"Deployment with ID {query.Id} does not exist"));

        if (deployment.Spec is null)
            return Result.Failure<DeploymentDuplicateDraft>(new BadRequestError("Deployment has no configuration to duplicate."));

        var warnings = new List<DuplicateDraftWarning>();
        var resourceBindings = await unitOfWork.ResourceBindings.GetEntriesAsync(ResourceBindingScope.Deployment, deployment.Id, cancellationToken);
        if (resourceBindings.Any())
        {
            warnings.Add(new DuplicateDraftWarning(
                "RESOURCE_BINDINGS_NOT_COPIED",
                "This deployment has resource-scoped configuration bindings. Recreate them after saving the duplicate.",
                "resourceBindings"));
        }

        if (HasLikelyHostBindMount(deployment.Spec.Volumes))
        {
            warnings.Add(new DuplicateDraftWarning(
                "HOST_BIND_MOUNT",
                "This deployment contains host paths that may not exist on another platform.",
                "spec.volumes"));
        }

        var draft = new DeploymentDuplicateDraft(
            Name: await GetAvailableDuplicateNameAsync(deployment.Name, deployment.PlatformId, cancellationToken),
            SourceName: deployment.Name,
            PlatformId: deployment.PlatformId,
            Description: deployment.Description,
            Spec: SanitizeSpec(deployment.Spec),
            TagIds: [.. deployment.Tags.Select(x => x.Id)],
            Warnings: warnings);

        return Result.Success(draft);
    }

    private static DeploymentSpec SanitizeSpec(DeploymentSpec spec)
    {
        var image = spec.Image switch
        {
            ExternalImage external => external with { ResolvedDigest = null },
            BuildImage build => build.ClearProvenance(),
            _ => spec.Image
        };

        return spec with { Image = image };
    }

    private async Task<string> GetAvailableDuplicateNameAsync(string sourceName, Guid platformId, CancellationToken cancellationToken)
    {
        for (var attempt = 1; attempt <= 100; attempt++)
        {
            var candidate = BuildDuplicateName(sourceName, attempt);
            if (!await unitOfWork.Deployments.ExistsAsync(candidate, platformId, cancellationToken))
                return candidate;
        }

        return BuildDuplicateName(sourceName, Guid.NewGuid().ToString("N")[..8]);
    }

    private static string BuildDuplicateName(string sourceName, int attempt)
        => BuildDuplicateName(sourceName, attempt == 1 ? "copy" : $"copy-{attempt}");

    private static string BuildDuplicateName(string sourceName, string suffix)
    {
        var fullSuffix = $"-{suffix}";
        var maxSourceLength = Math.Max(1, 64 - fullSuffix.Length);
        var trimmedSource = sourceName.Length <= maxSourceLength
            ? sourceName
            : sourceName[..maxSourceLength].TrimEnd('-', '_');

        if (string.IsNullOrWhiteSpace(trimmedSource))
            trimmedSource = "resource";

        return $"{trimmedSource}{fullSuffix}";
    }

    private static bool HasLikelyHostBindMount(IEnumerable<string>? volumes)
        => volumes?.Any(volume =>
        {
            var trimmed = volume.Trim();
            if (trimmed.StartsWith('/')) return true;
            var separatorIndex = trimmed.IndexOf(':');
            return separatorIndex > 0 && (trimmed[0] == '.' || trimmed[0] == '~');
        }) == true;
}
