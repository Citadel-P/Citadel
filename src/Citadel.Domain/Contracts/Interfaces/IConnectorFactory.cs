namespace Domain.Contracts.Interfaces;

/// <summary>
/// Contract for a factory that creates connectors for different platform types
/// </summary>
public interface IConnectorFactory<T>
{
    T GetConnector(PlatformConnectorType type);
}