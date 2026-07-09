namespace Domain.Contracts.Interfaces;

public interface IEdgeAgentSessionTerminator
{
    void Disconnect(Guid platformId, string reason);
}
