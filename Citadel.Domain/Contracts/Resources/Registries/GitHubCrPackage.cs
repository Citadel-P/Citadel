namespace Domain.Contracts.Resources.Registries;

public sealed record GitHubCrPackage(
    int Id,
    string Name,
    string? Url,
    string? PackageType,
    string? VersionCount,
    string? CreatedAt,
    string? UpdatedAt,
    string? HtmlUrl);
