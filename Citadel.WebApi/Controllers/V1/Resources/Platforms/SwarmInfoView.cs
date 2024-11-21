namespace WebApi.Controllers.V1.Resources.Platforms;

public class SwarmInfoView
{
    public Guid Id { get; set; }

    public string NodeID { get; set; }

    public string NodeAddr { get; set; }

    public string LocalNodeState { get; set; }

    public bool ControlAvailable { get; set; }

    public string Error { get; set; }

    public long Nodes { get; set; }

    public long Managers { get; set; }

    public IList<SwarmPeerView> RemoteManagers { get; set; }
}

public class SwarmPeerView(string NodeID, string Addr);