using Contracts.Broker.Models;

namespace Infrastructure.Entities;

public class SwarmInfo
{
    public Guid Id { get; }

    public Guid SystemInfoId { get; }

    public string NodeID { get; set; }

    public string NodeAddr { get; set; }

    public string LocalNodeState { get; set; }

    public bool ControlAvailable { get; set; }

    public string Error { get; set; }

    public long Nodes { get; set; }

    public long Managers { get; set; }

    public ICollection<SwarmPeer> RemoteManagers { get; set; } = [];

    public bool EqualsTo(SwarmInfoMessage message)
    {
        if (RemoteManagers.Count != message.RemoteManagers.Count)
        {
            return false;
        }

        foreach (var peer in message.RemoteManagers.OrderBy(s => s.NodeID))
        {
            if (RemoteManagers.FirstOrDefault(s => s.NodeID == peer.NodeID)?.EqualsTo(peer) == false)
            {
                return false;
            }
        }

        return NodeID == message.NodeID &&
                 NodeAddr == message.NodeAddr &&
                 LocalNodeState == message.LocalNodeState &&
                 ControlAvailable == message.ControlAvailable &&
                 Error == message.Error &&
                 Nodes == message.Nodes &&
                 Managers == message.Managers;
    }

    public void UpdateWith(SwarmInfoMessage message)
    {
        NodeID = message.NodeID;
        NodeAddr = message.NodeAddr;
        LocalNodeState = message.LocalNodeState;
        ControlAvailable = message.ControlAvailable;
        Error = message.Error;
        Nodes = message.Nodes;
        Managers = message.Managers;

        // Update the swarm peers
        List<SwarmPeer> peers = [];
        foreach (var peer in message.RemoteManagers)
        {
            peers.Add(new SwarmPeer()
            {
                NodeID = peer.NodeID,
                Addr = peer.Addr
            });
        }
        RemoteManagers = peers;
    }
}