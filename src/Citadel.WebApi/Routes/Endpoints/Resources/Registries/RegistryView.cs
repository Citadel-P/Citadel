using Application.Permissions;
using Domain;
using Domain.Entities.Registries;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryView(
    Guid Id,
    Guid CreatedByActorId, 
    string Name,
    RegistryStatus Status,
    string? Description, 
    string RegistryHost, 
    RegistryType Type,
    DateTime CreatedAt,
    IReadOnlyList<TagSummaryView> Tags,
    ResourceCapabilities? Capabilities = null)
{
    /// <summary>
    /// Default registry cannot be edited or deleted
    /// </summary>
    public bool IsDefault => Id == Constants.DefaultRegistryId;
    internal static async Task<RegistryView> Map(Registry registry, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(registry.Id, ResourceType.Registry);
        return Map(registry) with 
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }

    internal static RegistryView Map(Registry registry)
     => new (
         registry.Id,
         registry.CreatedByActorId,
         registry.Name, registry.Status, registry.Description,
         GetHost(registry.RegistryHost, registry.Configuration),
         GetType(registry.Configuration), registry.CreatedAt,
         [.. registry.Tags.Select(TagSummaryView.Map)]);

    private static RegistryType GetType(RegistryConfiguration config)
    {
        return config switch
        {
            CustomRegistry => RegistryType.Custom,
            GitHubRegistry => RegistryType.GitHub,
            DockerHubRegistry => RegistryType.DockerHub,
            _ => RegistryType.DockerHub
        };
    }

    private static string GetHost(string host, RegistryConfiguration config)
    {
        return config switch
        {
            GitHubRegistry => host + "/" + (config as GitHubRegistry)?.NameSpace,
            _ => host
        };
    }
}
