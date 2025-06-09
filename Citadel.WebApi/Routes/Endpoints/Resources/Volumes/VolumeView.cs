using Citadel.Agent.Volumes.V1;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumeView
    (
    string Id,
    string Driver,
    string Mountpoint,
    string CreatedAt,
    string Scope,
    bool InUse,
    UsageDataView? UsageData,
    Dictionary<string, string>? Labels,
    Dictionary<string, string>? Status,
    Dictionary<string, string>? Options
    )
{
    public static VolumesView Map(ListVolumeReply reply)
        => new([.. reply.Volumes.Select(Map)]);

    public static VolumeView Map(VolumeReply volume)
    {
        return new VolumeView(
            Id: volume.Name,
            Driver: volume.Driver,
            Mountpoint: volume.Mountpoint,
            CreatedAt: volume.CreatedAt,
            Scope: volume.Scope,
            InUse: volume.InUse,
            UsageData: volume.UsageData is null 
                ? null 
                : new UsageDataView(
                    volume.UsageData.Size, 
                    volume.UsageData.RefCount),
            Labels: volume.Labels?.ToDictionary(),
            Status: volume.Status?.ToDictionary(),
            Options: volume.Options?.ToDictionary());
    }
}