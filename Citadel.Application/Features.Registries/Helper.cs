using Infrastructure.Entities;
using Infrastructure;
using System.Text.Json;

namespace Application.Features.Registries;

public static class RegistryHelper
{
    public static string SerializeConfiguration(this IRegistryConfiguration configuration, RegistryDiscriminator discriminator)
    {
        string serializedCfg = string.Empty;
        switch (discriminator)
        {
            case RegistryDiscriminator.Azure:
                serializedCfg = JsonSerializer.Serialize((AzureRegistry)configuration);
                break;

            case RegistryDiscriminator.AWS:
                serializedCfg = JsonSerializer.Serialize((AWSRegistry)configuration);
                break;

            case RegistryDiscriminator.DockerHub:
                serializedCfg = JsonSerializer.Serialize((DockerHubRegistry)configuration);
                break;

            case RegistryDiscriminator.Gitlab:
                serializedCfg = JsonSerializer.Serialize((GitlabRegistry)configuration);
                break;

            case RegistryDiscriminator.Custom:
                serializedCfg = JsonSerializer.Serialize((CustomRegistry)configuration);
                break;
        }
        return serializedCfg;
    }
}