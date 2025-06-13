namespace Domain.Contracts.Resources.Networks;

public sealed record DeleteNetworkCommand(string PlatformAddress, string[] Ids);