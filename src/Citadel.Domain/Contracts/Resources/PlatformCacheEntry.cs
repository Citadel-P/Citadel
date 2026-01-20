using System.Collections.Immutable;

namespace Domain.Contracts.Resources;

public sealed record PlatformCacheEntry(Guid Id, string Address, PlatformConnectorType ConnectorType, ImmutableDictionary<string, Guid> Containers);
