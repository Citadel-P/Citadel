namespace Domain.Contracts.Interfaces;

public interface IConnectorFactory<T>
{
    T? GetConnector(Guid platformId);
    T GetConnector(PlatformConnectorType type);
}