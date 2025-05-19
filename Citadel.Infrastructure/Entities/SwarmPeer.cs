namespace Infrastructure.Entities;

public class SwarmPeer
{
    public Guid Id { get; private set; }
    public Guid SwarmInfoId { get; private set; }
    public string? NodeID { get; private set; }
    public string? Addr { get; private set; }
    
    /// <summary>
    /// EF navigation
    /// </summary>
    public SwarmInfo SwarmInfo { get; private set; } = null!;

    public static SwarmPeer Create(string nodeID, string addr)
        => new()
        {
            Id = Guid.CreateVersion7(),
            NodeID = nodeID,
            Addr = addr,
        };
}