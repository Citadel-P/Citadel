namespace Domain.Entities.Platforms;

public record KubernetesPlatformDescriptor(
    string? ClusterName,
    string? ClusterVersion,
    string? ApiServerUrl,
    string? Namespace
    ) : PlatformDescriptor;