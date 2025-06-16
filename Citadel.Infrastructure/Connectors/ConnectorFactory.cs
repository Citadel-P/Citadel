using Domain;
using Domain.Contracts.Interfaces;

namespace Infrastructure.Connectors;

internal class ConnectorFactory<T>(Func<PlatformConnectorType, T> connectorFactory) : IConnectorFactory<T> where T : class
{
    public T GetConnector(PlatformConnectorType type) 
        => connectorFactory(type);
}
