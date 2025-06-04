namespace Infrastructure.Entities;

public class KubernetesPlatformDescriptor : PlatformDescriptor
{
    public string? ClusterName { get; private set; }
    public string? ClusterVersion { get; private set; }
    public string? ApiServerUrl { get; private set; }
    public string? Namespace { get; private set; }
}