using Domain.Contracts.Resources.Networks;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record NetworksView(IEnumerable<DockerNetworkResult> Networks);
