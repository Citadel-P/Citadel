namespace Domain.Contracts.Resources;

public sealed record PlatformCacheEntry(Guid Id, string Address, PlatformConnectorType ConnectorType, Dictionary<string, Guid> Containers);
