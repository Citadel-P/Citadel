using Application.Features;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Queries;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, ResourceIdProperty = nameof(Id))]
public sealed record GetSwarmServiceDuplicateDraft(Guid Id) : IQuery<Result<SwarmServiceDuplicateDraft>>;

public sealed record SwarmServiceDuplicateDraft(
    string Name,
    string SourceName,
    Guid PlatformId,
    string? Description,
    SwarmServiceSpec Spec,
    IReadOnlyCollection<Guid> TagIds,
    IReadOnlyCollection<DuplicateDraftWarning> Warnings);

internal sealed class GetSwarmServiceDuplicateDraftHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetSwarmServiceDuplicateDraft, Result<SwarmServiceDuplicateDraft>>
{
    public async ValueTask<Result<SwarmServiceDuplicateDraft>> Handle(
        GetSwarmServiceDuplicateDraft query,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(query.Id, cancellationToken);
        if (service is null)
            return Result.Failure<SwarmServiceDuplicateDraft>(
                new NotFoundError($"Swarm Service with ID {query.Id} does not exist"));

        return Result.Success(new SwarmServiceDuplicateDraft(
            Name: await GetAvailableDuplicateNameAsync(service.Name, service.PlatformId, cancellationToken),
            SourceName: service.Name,
            PlatformId: service.PlatformId,
            Description: service.Description,
            Spec: SanitizeSpec(service.Spec),
            TagIds: [.. service.Tags.Select(static tag => tag.Id)],
            Warnings: []));
    }

    private static SwarmServiceSpec SanitizeSpec(SwarmServiceSpec spec)
    {
        var image = spec.Image switch
        {
            SwarmExternalImage external => external with { ResolvedDigest = null },
            SwarmBuildImage build => build with
            {
                ResolvedImageReference = null,
                ResolvedDigest = null,
                ResolvedBuildRunId = null,
            },
            _ => spec.Image,
        };

        return spec with { Image = image, Webhook = null };
    }

    private async Task<string> GetAvailableDuplicateNameAsync(
        string sourceName,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        for (var attempt = 1; attempt <= 100; attempt++)
        {
            var candidate = BuildDuplicateName(sourceName, attempt == 1 ? "copy" : $"copy-{attempt}");
            if (!await unitOfWork.SwarmServices.ExistsAsync(platformId, candidate, cancellationToken))
                return candidate;
        }

        return BuildDuplicateName(sourceName, Guid.NewGuid().ToString("N")[..8]);
    }

    private static string BuildDuplicateName(string sourceName, string suffix)
    {
        var fullSuffix = $"-{suffix}";
        var maxSourceLength = Math.Max(1, 64 - fullSuffix.Length);
        var trimmedSource = sourceName.Length <= maxSourceLength
            ? sourceName
            : sourceName[..maxSourceLength].TrimEnd('-', '_');

        return $"{(string.IsNullOrWhiteSpace(trimmedSource) ? "service" : trimmedSource)}{fullSuffix}";
    }
}
