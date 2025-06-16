namespace Domain.Contracts.Interfaces;

public interface IConnectorFactory<T>
{
    T GetConnector(PlatformConnectorType type);
}