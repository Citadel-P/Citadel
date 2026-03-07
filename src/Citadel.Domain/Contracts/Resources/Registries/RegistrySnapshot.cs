using Domain.Entities.Registries;

namespace Domain.Contracts.Resources.Registries;

public sealed record RegistrySnapshot(
    Guid Id,
    string Name,
    string Description,
    string RegistryHost,
    RegistryStatus Status,
    RegistryConfiguration Configuration);

public static class RegistrySnapshotExtensions
{
    public static RegistrySnapshot ToSnapshot(this Registry registry, Guid? id = null)
        => new (
            Id: id ?? registry.Id,
            Name: registry.Name,
            Description: registry.Description ?? string.Empty,
            RegistryHost: registry.RegistryHost,
            Status: registry.Status,
            Configuration: registry.Configuration.MaskSensitive());

    private static RegistryConfiguration MaskSensitive(this RegistryConfiguration configuration)
        => configuration switch
        {
            GitlabRegistry g => g with { PAT = MaskValue(g.PAT) },
            GitHubRegistry g => g with { PAT = MaskValue(g.PAT) },
            DockerHubRegistry d => d with { PAT = MaskValue(d.PAT) },
            CustomRegistry c => c with { Password = MaskValue(c.Password) },
            AzureRegistry a => a with { Password = MaskValue(a.Password)! },
            AWSRegistry a => a with { AccessKey = MaskValue(a.AccessKey)!, SecretAccessKey = MaskValue(a.SecretAccessKey)! },
            _ => configuration
        };

    private static string? MaskValue(string? value)
        => value is null ? null : new string('*', value.Length);
}
