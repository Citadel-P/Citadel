using Application.Services.Abstractions;
using Microsoft.AspNetCore.Authorization;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

public interface ITypedDockerDaemonHub
{
    Task ContainerEventReceived(ContainerView message, string @event);
}

[Authorize]
internal sealed class DockerDaemonHub(ISignalRConnectionTracker connectionTracker) : HubBase<ITypedDockerDaemonHub>(connectionTracker)
{
}