namespace Domain.Contracts.Resources;

public sealed record PlatformCacheEntry(string Address, PlatformConnectorType ConnectorType, Dictionary<string, Guid> Containers);
