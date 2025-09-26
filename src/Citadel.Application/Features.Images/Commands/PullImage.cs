using Application.Mappers;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;

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

    internal PullImageCommand ToConnectorCommand(string platformAddress, Registry registry)
    {
        string domainName = registry.Url.Replace("https://", "");
        switch (registry.Configuration)
        {
            case GitHubRegistry ghCfg:
                return new PullImageCommand
                    (
                        PlatformAddress: platformAddress,
                        RegistryName: RegistryName,
                        FromImage: $"{domainName}/{ghCfg.Name}/{RepositoryName}@{ImageTag}".ToLower(),
                        Repo: $"{domainName}/{ghCfg.Name}/{RepositoryName}@{ImageTag}".ToLower(),
                        FromSrc: registry.Url,
                        Tag: ImageTag,
                        Auth: ghCfg.GetRegistryAuth(registry.Url)
                    );

            case DockerHubRegistry dockerCfg:
                if (RegistryName == Registry.DefaultRegistryName)
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{ImageTag}:latest".ToLower()
                        );
                }
                else
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{dockerCfg.UserName}/{RepositoryName}:{ImageTag}".ToLower(),
                            Repo: $"{domainName}/{dockerCfg.UserName}/{RepositoryName}".ToLower(),
                            FromSrc: registry.Url,
                            Auth: dockerCfg.GetRegistryAuth(registry.Url)
                        );
                }

            default:
                throw new NotSupportedException("Unsupported registry configuration");
        }
    }
}

internal sealed class PullImageHandler(IConnectorFactory<IImageConnector> connectorFactory, IImageStreamManager imageStream, IUnitOfWork unitOfWork, 
    IPlatformContainerCache platformContainerCache, IBackgroundTaskQueue backgroundTaskQueue, IServiceScopeFactory scopeFactory) : IStreamCommandHandler<PullImage, PullImageResult>
{
    public async IAsyncEnumerable<PullImageResult> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var _))
        {
            var message = $"Platform with ID {command.PlatformId} not found or not available.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var registry = command.RegistryName == Registry.DefaultRegistryName
            ? Registry.DefaultRegistry() // Public Docker registry
            : await unitOfWork.Registries.GetByNameAsync(command.RegistryName, cancellationToken);

        if (registry == null)
        {
            var message = $"Registry configuration for '{command.RegistryName}' not found.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        await foreach (var reply in connector.PullImageProgressStreamAsync(command.ToConnectorCommand(platform.Address, registry), cancellationToken))
        {
            yield return reply;
        }

        backgroundTaskQueue.Enqueue(ct => PersistPulledImage(connector, scopeFactory, platform, registry, command.ImageTag, ct));
    }

    private async Task PersistPulledImage(IImageConnector connector, IServiceScopeFactory scopeFactory, PlatformCacheEntry paltform, Registry registry, string imageName, CancellationToken cancellationToken)
    {
        var imageResult = await connector.GetAsync(paltform.Address, imageName, cancellationToken);
        if (imageResult.IsSuccess(out var image))
        {
            var imageEntity = image.Map(paltform.Id, registry);
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            await uow.Images.AddOrUpdateAsync(imageEntity, cancellationToken);
            await uow.CommitAsync();

            await imageStream.SendImageInfo(paltform.Id, imageEntity);
        }
    }
}