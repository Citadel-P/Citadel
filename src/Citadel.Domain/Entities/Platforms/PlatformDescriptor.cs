using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Platforms;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DockerPlatformDescriptor), nameof(PlatformType.Docker))]
[JsonDerivedType(typeof(DockerSwarmPlatformDescriptor), nameof(PlatformType.DockerSwarm))]
[JsonDerivedType(typeof(KubernetesPlatformDescriptor), nameof(PlatformType.Kubernetes))]
public abstract record PlatformDescriptor
{

}
