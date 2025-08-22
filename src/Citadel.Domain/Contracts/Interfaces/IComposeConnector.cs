using Domain.Contracts.Resources.Compose;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing Docker Compose deployments.
/// </summary>
public interface IComposeConnector
{
    IAsyncEnumerable<ComposeDeploymentEvent> UpAsync(ComposeUpCommand command, CancellationToken cancellationToken);
    //Task DownAsync(ComposeDown command, CancellationToken cancellationToken);
    //IAsyncEnumerable<ComposeDeploymentEvent> RestartAsync(ComposeRestart command, CancellationToken cancellationToken);
}
