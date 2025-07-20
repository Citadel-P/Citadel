using Domain.Contracts.Resources.Volumes;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumesView(IEnumerable<DockerVolumeResult> Volumes);
