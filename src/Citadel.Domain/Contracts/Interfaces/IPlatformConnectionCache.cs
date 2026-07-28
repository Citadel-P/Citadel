namespace Domain.Contracts.Interfaces;

public interface IPlatformConnectionCache
{
    void Evict(string address);
}
