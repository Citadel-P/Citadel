using Domain.Entities.Registries;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

/// <summary>
/// Factory interface for resolving registry connector strategies based on registry baseType.
/// </summary>
internal interface IRegistryConnectorResolver
{
    IRegistryConnectorStrategy? Resolve(RegistryConfiguration baseType);
}

internal class RegistryConnectorResolver(IServiceProvider provider) : IRegistryConnectorResolver
{
    public IRegistryConnectorStrategy? Resolve(RegistryConfiguration baseType)
    {
        return baseType switch
        {
            CustomRegistry => provider.GetService<CustomRegistryConnectorStrategy>(),
            DockerHubRegistry => provider.GetService<DockerHubConnectorStrategy>(),
            GitHubRegistry => provider.GetService<GitHubConnectorStrategy>(),
            _ => null
        };
    }
}