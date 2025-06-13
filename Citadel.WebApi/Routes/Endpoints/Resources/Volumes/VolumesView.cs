using Domain.Contracts.Resources.Volumes;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record VolumesView(IEnumerable<DockerVolume> Volumes);
