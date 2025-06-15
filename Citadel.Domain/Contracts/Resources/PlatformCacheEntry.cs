namespace Domain.Contracts.Resources;

public sealed record PlatformCacheEntry(string PlatformAddress, PlatformConnectorType Type, Dictionary<string, Guid> Containers);
