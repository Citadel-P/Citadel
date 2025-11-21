using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;

namespace Application.Services;

/// <summary>
/// Strategy interface for connecting to different registry types.
/// </summary>
internal interface IRegistryConnectorStrategy
{
    RegistryType Type { get; }
    Task<(bool success, string? error)> CanConnectAsync(RegistryConfigurationBase config, CancellationToken ct);
}

/// <summary>
/// Strategy for connecting to DockerHub registry.
/// </summary>
internal class DockerHubConnectorStrategy(IDockerHubRegistryRepository dockerHubService) : IRegistryConnectorStrategy
{
    public RegistryType Type => RegistryType.DockerHub;

    public async Task<(bool success, string? error)> CanConnectAsync(RegistryConfigurationBase config, CancellationToken cancellationToken)
    {
        if (config is not DockerHubRegistry cfg)
        {
            return (false, "Invalid DockerHub config");
        }

        var (canConnect, errorMessage) = await dockerHubService.CanConnectAsync(cfg, cancellationToken);
        
        return !canConnect
            ? (false, errorMessage)
            : (true, null);
    }
}

/// <summary>
/// Strategy for connecting to Github registry.
/// </summary>
internal class GitHubConnectorStrategy(IGitHubCrRepository gitHubCrService) : IRegistryConnectorStrategy
{
    public RegistryType Type => RegistryType.GitHub;

    public async Task<(bool success, string? error)> CanConnectAsync(RegistryConfigurationBase config, CancellationToken cancellationToken)
    {
        if (config is not GitHubRegistry cfg)
        {
            return (false, "Invalid GitHub config");
        }

        var (canConnect, errorMessage) = await gitHubCrService.CanConnectAsync(cfg, cancellationToken);
        
        return !canConnect
            ? (false, errorMessage)
            : (true, null);
    }
}

internal class CustomRegistryConnectorStrategy() : IRegistryConnectorStrategy
{
    public RegistryType Type => RegistryType.Custom;
    public async Task<(bool success, string? error)> CanConnectAsync(RegistryConfigurationBase config, CancellationToken cancellationToken)
    {
        if (config is not CustomRegistry)
        {
            return (false, "Invalid Custom Registry config");
        }
        // For custom registries, we assume the connection is always successful.
        return (true, null);
    }
}
