using Domain;
using Domain.Entities;
using Domain.Entities.Registries;
using Hosting.Common;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryView(
    Guid Id,
    Guid CreatedByActorId, 
    string Name,
    RegistryStatus Status,
    string? Description, 
    string RegistryHost, 
    RegistryType Type,
    DateTime CreatedAt)
{
    /// <summary>
    /// Default registry cannot be edited or deleted
    /// </summary>
    public bool IsDefault => CreatedByActorId == Constants.SystemId;
    internal static RegistryView Map(Registry registry) => new(registry.Id, registry.CreatedByActorId, registry.Name, registry.Status, registry.Description, registry.RegistryHost, GetType(registry.Configuration), registry.CreatedAt);

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