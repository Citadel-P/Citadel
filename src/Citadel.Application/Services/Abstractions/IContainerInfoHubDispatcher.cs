using Domain.Contracts.Resources.Containers;

namespace Application.Services.Abstractions;

public interface IContainerInfoHubDispatcher
{
    Task SendContainerInfo(DockerContainer container, CancellationToken cancellationToken);
}