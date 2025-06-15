using Domain;
using Domain.Contracts.Interfaces;
using Infrastructure.Services;

namespace Infrastructure.Connectors;

internal class ConnectorFactory<T>(Func<PlatformConnectorType, T> connectorFactory, IPlatformContainerCache cache) : IConnectorFactory<T> where T : class
{
    public T? GetConnector(Guid platformId)
    {
        if (cache.TryGetCacheEntry(platformId, out var entry)) 
        {
            return connectorFactory(entry.Type);
        }
        return null;
    }

    public T GetConnector(PlatformConnectorType type) 
        => connectorFactory(type);
}
