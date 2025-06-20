namespace Domain.Contracts.Resources.Networks;

public sealed record DeleteDockerNetworkCommand(string PlatformAddress, string[] Ids);