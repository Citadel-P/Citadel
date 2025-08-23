using System.Text.Json.Serialization;
using Domain.Entities.Registries;

namespace Domain.Entities;

[method: JsonConstructor]
public class Registry(
    string name,
    string url,
    RegistryType type,
    RegistryConfigurationBase configuration)
{
    public static readonly string DefaultRegistryName = "Docker Hub";
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string Url { get; private set; } = url;
    public DateTime Created { get; private set; } = DateTime.UtcNow;
    public RegistryType Type { get; private set; } = type;
    public RegistryConfigurationBase Configuration { get; private set; } = configuration;

    public void PartialUpdate(string? name = null, string? url = null, RegistryConfigurationBase? configuration = null)
    {
        if (name != null) Name = name;
        if (url != null) Url = url;
        if (configuration != null) Configuration = configuration;
    }

    public static Registry FromPersistence(
        Guid id,
        string name,
        string url,
        DateTime created, 
        RegistryType type,
        RegistryConfigurationBase configuration)
    {
        return new Registry(name, url, type, configuration)
        {
            Id = id,
            Created = created
        };
    }

    public static Registry DefaultRegistry()
    {
        var registry = new Registry(
            name: DefaultRegistryName,
            url: "https://hub.docker.com",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry())
        {
            Id = Guid.Empty,
            Created = DateTime.MinValue
        };

        return registry;
    }
}