using System.Text.Json.Serialization;

namespace Domain.Entities.Registries;

[method: JsonConstructor]
public class Registry(
    string name,
    string registryHost,
    RegistryStatus status,
    Guid createdByActorId,
    RegistryConfiguration configuration,
    string? description = null) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public string RegistryHost { get; private set; } = registryHost;
    public RegistryStatus Status { get; private set; } = status;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    public RegistryConfiguration Configuration { get; private set; } = configuration;

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        string? registryHost = null,
        RegistryStatus? status = null,
        RegistryConfiguration? configuration = null)
    {
        if (name != null) Name = name;
        if (status != null) Status = status.Value;
        if (description != null) Description = description;
        if (registryHost != null) RegistryHost = registryHost;
        if (configuration != null) Configuration = configuration;

    }

    public static Registry FromPersistence(
        Guid id,
        string name,
        string? description,
        RegistryStatus status,
        string registryHost,
        DateTime createdAt, 
        Guid createdByActorId,
        RegistryConfiguration configuration)
    {
        return new Registry(name, registryHost, status, createdByActorId, configuration, description)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }
}