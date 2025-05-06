using Agent.Server.Volumes;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumeView
    (
    string Name,
    string Driver,
    string Mountpoint,
    string CreatedAt,
    string Scope,
    bool InUse,
    Dictionary<string, string> Labels,
    Dictionary<string, string> Status,
    Dictionary<string, string> Options
    )
{
    public static VolumesView Map(VolumeListReply reply)
        => new([.. reply.Volumes.Select(Map)]);

    public static VolumeView Map(VolumeReply volume)
    {
        return new VolumeView(
            Name: volume.Name,
            Driver: volume.Driver,
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Scope: volume.Scope,
            InUse: volume.InUse,
            Labels: volume.Labels?.ToDictionary(),
            Status: volume.Status?.ToDictionary(),
            Options: volume.Options?.ToDictionary());
    }
}