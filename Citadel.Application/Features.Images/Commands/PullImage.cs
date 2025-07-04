using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Mediator;

namespace Application.Features.Images.Commands;

/// <summary>
/// Command to pull an image from a registry.
/// </summary>
public sealed record PullImage(Guid PlatformId, string RegistryName, string RepositoryName, string ImageTag) : IStreamCommand<PullImageResult>
{
    internal class Validator : AbstractValidator<PullImage>
    {
        public Validator()
        {
            RuleFor(s => s.ImageTag).NotEmpty().NotNull();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
        }
    }

    internal PullImageCommand ToConnectorCommand(string platformAddress, RegistryConfigurationBase registryCfg)
    {
        string domainName = registryCfg.RegistryUrl.Replace("https://", "");
        switch (registryCfg)
        {
            case GitHubRegistry ghCfg:
                return new PullImageCommand
                    (
                        PlatformAddress: platformAddress,
                        RegistryName: RegistryName,
                        FromImage: $"{domainName}/{ghCfg.Name}/{RepositoryName}@{ImageTag}".ToLower(),
                        Repo: $"{domainName}/{ghCfg.Name}/{RepositoryName}@{ImageTag}".ToLower(),
                        FromSrc: ghCfg.RegistryUrl,
                        Tag: ImageTag,
                        Auth: ghCfg.GetRegistryAuth()
                    );

            case DockerHubRegistry dockerCfg:
                if (RegistryName == Registry.DefaultRegistryName)
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{ImageTag}:latest".ToLower(),
                            FromSrc: dockerCfg.RegistryUrl,
                            Repo: domainName,
                            Auth: string.Empty
                        );
                }
                else
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{dockerCfg.UserName}/{RepositoryName}:{ImageTag}".ToLower(),
                            Repo: $"{domainName}/{dockerCfg.UserName}/{RepositoryName}".ToLower(),
                            FromSrc: dockerCfg.RegistryUrl,
                            Auth: dockerCfg.GetRegistryAuth()
                        );
                }

            default:
                throw new NotSupportedException("Unsupported registry configuration");
        }
    }
}

internal sealed class PullImageHandler(IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> connectorFactory) 
    : IStreamCommandHandler<PullImage, PullImageResult>
{
    public async IAsyncEnumerable<PullImageResult> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(command.PlatformId, cancellationToken);
        if (platform is null)
        {
            yield break;
        }

        var registryConfiguration = command.RegistryName == Registry.DefaultRegistryName
            ? Registry.DefaultRegistry().Configuration // Public Docker registry
            : await unitOfWork.Registries.GetRegistryConfigurationAsync(command.RegistryName, cancellationToken);

        if (registryConfiguration == null)
        {
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.Value.ConnectorType);
        await foreach (var reply in connector.PullImageProgressStreamAsync(command.ToConnectorCommand(platform.Value.Address, registryConfiguration), cancellationToken))
        {
            yield return reply;
        }
    }
}