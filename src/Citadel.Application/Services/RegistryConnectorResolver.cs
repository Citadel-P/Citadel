using Domain;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

/// <summary>
/// Factory interface for resolving registry connector strategies based on registry type.
/// </summary>
internal interface IRegistryConnectorResolver
{
    IRegistryConnectorStrategy? Resolve(RegistryType type);
}

internal class RegistryConnectorResolver(IServiceProvider provider) : IRegistryConnectorResolver
{
    public IRegistryConnectorStrategy? Resolve(RegistryType type)
    {
        return type switch
        {
            RegistryType.Custom => provider.GetService<CustomRegistryConnectorStrategy>(),
            RegistryType.DockerHub => provider.GetService<DockerHubConnectorStrategy>(),
            RegistryType.GitHub => provider.GetService<GitHubConnectorStrategy>(),
            _ => null
        };
    }
}