using Domain.Entities.Registries;
using Hosting.Common;

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
            GitlabRegistry g => g with { PAT = g.PAT != null ? g.PAT.MaskValue() : "" },
            GitHubRegistry g => g with { PAT = g.PAT.MaskValue() },
            DockerHubRegistry d => d with { PAT = d.PAT.MaskValue() },
            CustomRegistry c => c with { Password = c.Password.MaskValue() },
            AzureRegistry a => a with { Password = a.Password.MaskValue()! },
            AWSRegistry a => a with { AccessKey = a.AccessKey.MaskValue()!, SecretAccessKey = a.SecretAccessKey.MaskValue()! },
            _ => configuration
        };
}
