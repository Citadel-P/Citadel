using System.Text.Json.Serialization;
using Domain.Entities.Registries;

namespace Domain.Entities;

[method: JsonConstructor]
public class Registry(
    string name,
    string registryHost,
    RegistryConfigurationBase configuration)
{
    public static readonly string DefaultRegistryName = "Docker Hub";
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string RegistryHost { get; private set; } = registryHost;
    public DateTime Created { get; private set; } = DateTime.UtcNow;
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
        string registryHost,
        DateTime created, 
        RegistryConfigurationBase configuration)
    {
        return new Registry(name, registryHost, configuration)
        {
            Id = id,
            Created = created
        };
    }

    public static Registry DefaultRegistry()
    {
        var registry = new Registry(
            name: DefaultRegistryName,
            registryHost: "hub.docker.com",
            configuration: new DockerHubRegistry())
        {
            Id = Guid.Empty,
            Created = DateTime.MinValue
        };

        return registry;
    }
}