namespace Domain.Contracts.Resources.Platforms;

public struct PlatformHealth(bool Healthy)
{
    public bool Healthy { get; set; } = Healthy;
}
