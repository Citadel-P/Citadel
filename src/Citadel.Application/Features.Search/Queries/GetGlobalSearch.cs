using Application.Features.Search.Models;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Search;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Search.Queries;

public sealed record GetGlobalSearchQuery(
    string? Query,
    string? Types,
    int LimitPerType = 5) : IQuery<Result<GlobalSearchResponse>>;

internal sealed class GetGlobalSearchQueryHandler(
    IUserContextAccessor userContextAccessor,
    IUnitOfWork unitOfWork) : IQueryHandler<GetGlobalSearchQuery, Result<GlobalSearchResponse>>
{
    private const int MinimumQueryLength = 2;
    private const int MaximumQueryLength = 100;
    private const int MaximumLimitPerType = 10;
    private const int MaximumResults = 40;

    private static readonly GlobalSearchResourceType[] AllTypes = Enum.GetValues<GlobalSearchResourceType>();
    private static readonly GlobalSearchCategory[] CategoryOrder = Enum.GetValues<GlobalSearchCategory>();

    public async ValueTask<Result<GlobalSearchResponse>> Handle(
        GetGlobalSearchQuery query,
        CancellationToken cancellationToken)
    {
        var normalizedQuery = query.Query?.Trim() ?? string.Empty;
        if (normalizedQuery.Length is < MinimumQueryLength or > MaximumQueryLength)
        {
            return Result.Failure<GlobalSearchResponse>(
                new BadRequestError($"q must be between {MinimumQueryLength} and {MaximumQueryLength} characters."));
        }

        if (query.LimitPerType is < 1 or > MaximumLimitPerType)
        {
            return Result.Failure<GlobalSearchResponse>(
                new BadRequestError($"limitPerType must be between 1 and {MaximumLimitPerType}."));
        }

        var parsedTypes = ParseTypes(query.Types);
        if (parsedTypes.IsFailure(out var typeError))
        {
            return Result.Failure<GlobalSearchResponse>(typeError);
        }

        _ = parsedTypes.IsSuccess(out var searchTypes);

        var user = userContextAccessor.Current;
        if (user is null || !user.IsAuthenticated || user.UserId == Guid.Empty)
        {
            return Result.Failure<GlobalSearchResponse>(new UnauthorizedError("Authentication is required."));
        }

        var resourceTypes = new ResourceType[searchTypes.Length];
        for (var i = 0; i < searchTypes.Length; i++)
        {
            resourceTypes[i] = ToResourceType(searchTypes[i]);
        }

        var matches = await unitOfWork.GlobalSearch.SearchAsync(
            user.UserId,
            user.IsAdmin,
            resourceTypes,
            normalizedQuery,
            query.LimitPerType,
            MaximumResults,
            cancellationToken);

        return Result.Success(MapResponse(normalizedQuery, matches));
    }

    private static Result<GlobalSearchResourceType[]> ParseTypes(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return Result.Success(AllTypes);
        }

        var values = value.Split(',', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
        if (values.Length == 0)
        {
            return Result.Failure<GlobalSearchResourceType[]>(
                new BadRequestError("types must contain at least one supported resource type."));
        }

        var types = new List<GlobalSearchResourceType>(values.Length);
        foreach (var item in values)
        {
            if (!Enum.TryParse<GlobalSearchResourceType>(item, ignoreCase: true, out var parsed))
            {
                return Result.Failure<GlobalSearchResourceType[]>(
                    new BadRequestError($"Search resource type '{item}' is not supported."));
            }

            if (!types.Contains(parsed))
            {
                types.Add(parsed);
            }
        }

        return Result.Success(types.ToArray());
    }

    private static GlobalSearchResponse MapResponse(
        string query,
        IReadOnlyList<GlobalSearchMatch> matches)
    {
        var itemsByCategory = new Dictionary<GlobalSearchCategory, List<GlobalSearchItem>>();

        foreach (var match in matches)
        {
            var resourceType = ToSearchResourceType(match.ResourceType);
            var category = ToCategory(resourceType);
            if (!itemsByCategory.TryGetValue(category, out var items))
            {
                items = [];
                itemsByCategory.Add(category, items);
            }

            GlobalSearchStatus? status = null;
            if (!string.IsNullOrWhiteSpace(match.Status))
            {
                status = new GlobalSearchStatus(
                    FormatStatusLabel(match.Status),
                    ToStatusTone(match.Status));
            }

            GlobalSearchParent? parent = null;
            if (match.ParentId.HasValue &&
                match.ParentResourceType.HasValue &&
                !string.IsNullOrWhiteSpace(match.ParentName))
            {
                parent = new GlobalSearchParent(
                    match.ParentId.Value,
                    ToSearchResourceType(match.ParentResourceType.Value),
                    match.ParentName);
            }

            items.Add(new GlobalSearchItem(
                match.Id,
                resourceType,
                match.Name,
                match.SecondaryText,
                status,
                parent));
        }

        var groups = new List<GlobalSearchGroup>(itemsByCategory.Count);
        foreach (var category in CategoryOrder)
        {
            if (itemsByCategory.TryGetValue(category, out var items) && items.Count > 0)
            {
                groups.Add(new GlobalSearchGroup(category, items));
            }
        }

        return new GlobalSearchResponse(query, groups);
    }

    private static ResourceType ToResourceType(GlobalSearchResourceType type)
        => type switch
        {
            GlobalSearchResourceType.Platform => ResourceType.Platform,
            GlobalSearchResourceType.Stack => ResourceType.Stack,
            GlobalSearchResourceType.Deployment => ResourceType.Deployment,
            GlobalSearchResourceType.GitRepository => ResourceType.GitRepository,
            GlobalSearchResourceType.Registry => ResourceType.Registry,
            GlobalSearchResourceType.AutomationAction => ResourceType.AutomationAction,
            GlobalSearchResourceType.BackupPolicy => ResourceType.BackupPolicy,
            GlobalSearchResourceType.BackupRepository => ResourceType.BackupRepository,
            GlobalSearchResourceType.Build => ResourceType.Build,
            GlobalSearchResourceType.BuildAgentPool => ResourceType.BuildAgentPool,
            GlobalSearchResourceType.SwarmService => ResourceType.SwarmService,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };

    private static GlobalSearchResourceType ToSearchResourceType(ResourceType type)
        => type switch
        {
            ResourceType.Platform => GlobalSearchResourceType.Platform,
            ResourceType.Stack => GlobalSearchResourceType.Stack,
            ResourceType.Deployment => GlobalSearchResourceType.Deployment,
            ResourceType.GitRepository => GlobalSearchResourceType.GitRepository,
            ResourceType.Registry => GlobalSearchResourceType.Registry,
            ResourceType.AutomationAction => GlobalSearchResourceType.AutomationAction,
            ResourceType.BackupPolicy => GlobalSearchResourceType.BackupPolicy,
            ResourceType.BackupRepository => GlobalSearchResourceType.BackupRepository,
            ResourceType.Build => GlobalSearchResourceType.Build,
            ResourceType.BuildAgentPool => GlobalSearchResourceType.BuildAgentPool,
            ResourceType.SwarmService => GlobalSearchResourceType.SwarmService,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };

    private static GlobalSearchCategory ToCategory(GlobalSearchResourceType type)
        => type switch
        {
            GlobalSearchResourceType.Platform => GlobalSearchCategory.Platforms,
            GlobalSearchResourceType.Stack => GlobalSearchCategory.Stacks,
            GlobalSearchResourceType.Deployment => GlobalSearchCategory.Deployments,
            GlobalSearchResourceType.GitRepository => GlobalSearchCategory.Repositories,
            GlobalSearchResourceType.Registry => GlobalSearchCategory.Registries,
            GlobalSearchResourceType.AutomationAction => GlobalSearchCategory.Automations,
            GlobalSearchResourceType.BackupPolicy or
                GlobalSearchResourceType.BackupRepository => GlobalSearchCategory.Backups,
            GlobalSearchResourceType.Build or
                GlobalSearchResourceType.BuildAgentPool => GlobalSearchCategory.Builds,
            GlobalSearchResourceType.SwarmService => GlobalSearchCategory.SwarmServices,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };

    private static SearchStatusTone ToStatusTone(string status)
        => status switch
        {
            "Online" or "Healthy" or "Active" or "Ready" or "Succeeded" or "Enabled"
                => SearchStatusTone.Positive,
            "Offline" or "Failed" or "Invalid" or "TimedOut" or "Interrupted"
                => SearchStatusTone.Negative,
            "Degraded" or "Deprecated" or "Pending" or "Queued" or "Paused" or "Applying" or
                "SucceededWithWarnings"
                => SearchStatusTone.Warning,
            "Created" or "Uninitialized" or "Preparing" or "Processing" or "Provisioning" or "Running" or
                "ApplyingRetention"
                => SearchStatusTone.Info,
            _ => SearchStatusTone.Neutral
        };

    private static string FormatStatusLabel(string status)
        => status switch
        {
            "NotTested" => "Not tested",
            "TimedOut" => "Timed out",
            "SucceededWithWarnings" => "Succeeded with warnings",
            "ApplyingRetention" => "Applying retention",
            _ => status
        };
}
