using Domain;
using Domain.Entities;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryView(Guid Id, string Name, string RegistryHost, RegistryType Type, DateTime Created)
{
    /// <summary>
    /// Default registry cannot be edited or deleted
    /// </summary>
    public bool IsDefault => Id == Guid.Empty;
    internal static RegistryView Map(Registry registry) => new(registry.Id, registry.Name, registry.RegistryHost, GetType(registry.Configuration), registry.Created);

    private static RegistryType GetType(RegistryConfigurationBase config)
    {
        return config switch
        {
            CustomRegistry => RegistryType.Custom,
            GitHubRegistry => RegistryType.GitHub,
            DockerHubRegistry => RegistryType.DockerHub,
            _ => RegistryType.DockerHub
        };
    }
}