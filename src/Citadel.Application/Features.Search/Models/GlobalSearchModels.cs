namespace Application.Features.Search.Models;

public enum GlobalSearchResourceType
{
    Platform,
    Stack,
    Deployment,
    GitRepository,
    Registry,
    AutomationAction,
    BackupPolicy,
    BackupRepository,
    Build,
    BuildAgentPool
}

public enum GlobalSearchCategory
{
    Platforms,
    Stacks,
    Deployments,
    Repositories,
    Registries,
    Automations,
    Backups,
    Builds
}

public enum SearchStatusTone
{
    Positive,
    Negative,
    Warning,
    Info,
    Neutral
}

public sealed record GlobalSearchStatus(string Label, SearchStatusTone Tone);

public sealed record GlobalSearchParent(
    Guid Id,
    GlobalSearchResourceType ResourceType,
    string Name);

public sealed record GlobalSearchItem(
    Guid Id,
    GlobalSearchResourceType ResourceType,
    string Name,
    string? SecondaryText,
    GlobalSearchStatus? Status,
    GlobalSearchParent? Parent);

public sealed record GlobalSearchGroup(
    GlobalSearchCategory Category,
    IReadOnlyList<GlobalSearchItem> Items);

public sealed record GlobalSearchResponse(
    string Query,
    IReadOnlyList<GlobalSearchGroup> Groups);
