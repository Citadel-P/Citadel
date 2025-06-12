namespace Domain.Contracts.Resources.Registries;

public sealed record DockerHubRepositoryInfo(
    string? Name,
    string? Namespace,
    DateTime LastUpdated,
    bool IsPrivate,
    bool IsTrusted,
    bool IsAutomated,
    int PullCount );
