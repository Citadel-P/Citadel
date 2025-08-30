using Domain;
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryView(Guid Id, string Name, string Url, RegistryType Type, DateTime Created)
{
    /// <summary>
    /// Default registry cannot be edited or deleted
    /// </summary>
    public bool IsDefault => Id == Guid.Empty;
    internal static RegistryView Map(Registry registry) => new(registry.Id, registry.Name, registry.Url, registry.Type, registry.Created);
}