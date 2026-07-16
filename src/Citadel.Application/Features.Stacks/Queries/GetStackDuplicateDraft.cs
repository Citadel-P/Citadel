using Application.Features;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetStackDuplicateDraft(Guid Id) : IQuery<Result<StackDuplicateDraft>>;

public sealed record StackDuplicateDraft(
    string Name,
    string SourceName,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    StackDriftPolicy DriftPolicy,
    IReadOnlyCollection<Guid> TagIds,
    IReadOnlyCollection<DuplicateDraftWarning> Warnings);

internal sealed class GetStackDuplicateDraftHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetStackDuplicateDraft, Result<StackDuplicateDraft>>
{
    public async ValueTask<Result<StackDuplicateDraft>> Handle(GetStackDuplicateDraft query, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(query.Id, cancellationToken);
        if (stack is null)
            return Result.Failure<StackDuplicateDraft>(new NotFoundError($"Stack with ID {query.Id} does not exist"));

        var release = stack.CurrentStackRelease;
        if (release?.Spec is null)
            return Result.Failure<StackDuplicateDraft>(new BadRequestError("Stack has no current release configuration to duplicate."));

        var warnings = new List<DuplicateDraftWarning>();
        var resourceBindings = await unitOfWork.ResourceBindings.GetEntriesAsync(ResourceBindingScope.Stack, stack.Id, cancellationToken);
        if (resourceBindings.Any())
        {
            warnings.Add(new DuplicateDraftWarning(
                "RESOURCE_BINDINGS_NOT_COPIED",
                "This stack has resource-scoped configuration bindings. Recreate them after saving the duplicate.",
                "resourceBindings"));
        }

        var sanitizedSpec = SanitizeSpec(release.Spec, warnings);
        if (HasExternalNetworkHint(sanitizedSpec))
        {
            warnings.Add(new DuplicateDraftWarning(
                "EXTERNAL_NETWORK",
                "This stack references an external Docker network that may not exist on another platform.",
                "spec"));
        }

        var draft = new StackDuplicateDraft(
            Name: await GetAvailableDuplicateNameAsync(stack.Name, cancellationToken),
            SourceName: stack.Name,
            PlatformId: release.PlatformId,
            Description: stack.Description,
            StackSource: stack.StackSource,
            Spec: sanitizedSpec,
            DriftPolicy: stack.DriftPolicy,
            TagIds: [.. stack.Tags.Select(x => x.Id)],
            Warnings: warnings);

        return Result.Success(draft);
    }

    private static StackSpec SanitizeSpec(StackSpec spec, ICollection<DuplicateDraftWarning> warnings)
    {
        if (!string.IsNullOrWhiteSpace(spec.ProjectName))
        {
            warnings.Add(new DuplicateDraftWarning(
                "COMPOSE_PROJECT_NAME_NOT_COPIED",
                "The Compose project name was intentionally omitted so the duplicate can deploy with its own stack name.",
                "spec.projectName"));
        }

        if (spec is GitStack { Webhook.Secret: not null } git)
        {
            warnings.Add(new DuplicateDraftWarning(
                "WEBHOOK_SECRET_NOT_COPIED",
                "The stack webhook secret was intentionally omitted. Generate a new secret after saving the duplicate.",
                "spec.webhook.secret"));

            return git with
            {
                ProjectName = null,
                Webhook = git.Webhook with { Secret = null }
            };
        }

        return spec switch
        {
            ManualStack manual => manual with { ProjectName = null },
            GitStack gitSpec => gitSpec with { ProjectName = null },
            _ => spec
        };
    }

    private async Task<string> GetAvailableDuplicateNameAsync(string sourceName, CancellationToken cancellationToken)
    {
        for (var attempt = 1; attempt <= 100; attempt++)
        {
            var candidate = BuildDuplicateName(sourceName, attempt);
            if (!await unitOfWork.Stacks.ExistsAsync(candidate, cancellationToken))
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

    private static bool HasExternalNetworkHint(StackSpec spec)
        => spec is ManualStack manual
            && manual.ComposeFile.Contains("external: true", StringComparison.OrdinalIgnoreCase);
}
