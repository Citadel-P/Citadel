namespace Domain.Contracts.Resources.Networks;

public sealed record ListNetworksCommand(
    string PlatformAddress,
    string? Id = null,
    string? Name = null,
    string? Driver = null,
    bool? Dangling = null);
