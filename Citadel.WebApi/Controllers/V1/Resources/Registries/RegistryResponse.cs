using Infrastructure.Entities;
using System.Text.Json.Serialization;

namespace WebApi.Controllers.V1.Resources.Registries;

public sealed record RegistryResponse(Guid Id, string Name, string Url, DateTime Created, IRegistryConfiguration Configuration)
{
    [JsonPropertyName("Id")]
    public Guid Id { get; init; } = Id;

    internal static RegistryResponse Map(Registry registry) => new(registry.Id, registry.Name, registry.Url, registry.Created, null);
}