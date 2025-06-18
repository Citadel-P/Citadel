using System.Text.Json.Serialization;
using Domain.Entities.Registries;

namespace Domain.Entities;

public class Registry
{
    private Registry() { /* For EF Core */ }

    [method: JsonConstructor]
    public Registry(
        string name,
        string url,
        RegistryType type,
        RegistryConfigurationBase configuration)
    {
        Id = Guid.CreateVersion7();
        Name = name;
        Url = url;
        Type = type;
        Created = DateTime.UtcNow;
        Configuration = configuration;
    }

    public static readonly string DefaultRegistryName = "Docker Hub";
    public Guid Id { get; private set; }
    public string Name { get; private set; }
    public string Url { get; private set; }
    public DateTime Created { get; private set; }
    public RegistryType Type { get; private set; }
    public RegistryConfigurationBase Configuration { get; private set; }

    public void PartialUpdate(string? name = null, string? url = null, RegistryConfigurationBase? configuration = null)
    {
        if (name != null) Name = name;
        if (url != null) Url = url;
        if (configuration != null) Configuration = configuration;
    }

    public static Registry DefaultRegistry()
    {
        var registry = new Registry(
            name: DefaultRegistryName,
            url: "https://hub.docker.com/",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry())
        {
            Id = Guid.Empty,
            Created = DateTime.MinValue // Set to a default value for the default registry
        };

        return registry;
    }
}