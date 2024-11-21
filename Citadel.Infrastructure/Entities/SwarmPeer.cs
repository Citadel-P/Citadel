using Contracts.Broker.Models;

namespace Infrastructure.Entities;

public class SwarmPeer
{
    public Guid Id { get; }

    public Guid SwarmInfoId { get; }

    public string NodeID { get; set; }

    public string Addr { get; set; }

    public bool EqualsTo(SwarmPeerMessage message)
    {
        return NodeID == message.NodeID &&
            Addr == message.Addr;
    }
}