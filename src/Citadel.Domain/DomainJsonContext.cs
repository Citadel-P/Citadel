using System.Text.Json.Serialization;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;

namespace Domain;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(Platform))]
[JsonSerializable(typeof(PlatformStat))]
[JsonSerializable(typeof(ICollection<PlatformStat>))]
[JsonSerializable(typeof(DockerPlatformDescriptor))]
[JsonSerializable(typeof(DockerSwarmPlatformDescriptor))]
[JsonSerializable(typeof(KubernetesPlatformDescriptor))]
[JsonSerializable(typeof(ICollection<SwarmPeer>))]
[JsonSerializable(typeof(PlatformDescriptor))]
public partial class PlatformJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(Registry))]
[JsonSerializable(typeof(AWSRegistry))]
[JsonSerializable(typeof(AzureRegistry))]
[JsonSerializable(typeof(GitlabRegistry))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(GitHubRegistry))]
[JsonSerializable(typeof(RegistryConfigurationBase))]
public partial class RegistryJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(IReadOnlyCollection<ContainerPort>))]
[JsonSerializable(typeof(IEnumerable<ContainerPort>))]
[JsonSerializable(typeof(List<ContainerPort>))]
public partial class ContainerPortsContext : JsonSerializerContext
{
}