namespace Domain.Contracts.Resources.Registries;

public sealed record GitHubCrPackageVersion(
    int Id,
    string Url,
    string Name,
    string? HtmlUrl,
    string? CreatedAt,
    string? UpdatedAt,
    string? PackageHtmlUrl,
    GitHubCrPackageVersionMetadata? Metadata
    );

public sealed record GitHubCrPackageVersionMetadata(GitHubCrPackageVersionContainerMetadata? Container);
public sealed record GitHubCrPackageVersionContainerMetadata(IEnumerable<string>? Tags);