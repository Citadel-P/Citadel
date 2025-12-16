using Domain.Entities.Registries;
using System.Text.Json.Serialization;

namespace Domain.Entities;

[method: JsonConstructor]
public class Registry(
    string name,
    string registryHost,
    Guid createdByActorId,
    RegistryConfigurationBase configuration,
    string? description = null) : AuditedEntity(createdByActorId)
{
    public static readonly string DefaultRegistryName = "Docker Hub";
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public string RegistryHost { get; private set; } = registryHost;

    public RegistryConfigurationBase Configuration { get; private set; } = configuration;

    public void PartialUpdate(string? name = null, string? registryHost = null, RegistryConfigurationBase? configuration = null)
    {
        if (name != null) Name = name;
        if (registryHost != null) RegistryHost = registryHost;
        if (configuration != null) Configuration = configuration;

    }

    public static Registry FromPersistence(
        Guid id,
        string name,
        string? description,
        string registryHost,
        DateTime createdAt, 
        Guid createdByActorId,
        RegistryConfigurationBase configuration)
    {
        return new Registry(name, registryHost, createdByActorId, configuration, description)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }
}