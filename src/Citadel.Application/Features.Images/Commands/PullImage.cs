using Application.Mappers;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
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
            
            case CustomRegistry customCfg:
                return new PullImageCommand
                (
                    PlatformAddress: platformAddress,
                    FromImage: $"{domainName}/{ImageTag}".ToLower(),
                    Repo: $"{domainName}".ToLower(),
                    FromSrc: registry.Url,
                    Auth: customCfg.AuthEnabled == true ? customCfg.GetRegistryAuth(registry.Url) : null
                );

            case DockerHubRegistry dockerCfg:
                if (RegistryName == Registry.DefaultRegistryName)
                {
                    var imageDefaultTag = ImageTag.Split(':').Length == 1 ? $"{ImageTag}:latest" : ImageTag;
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: imageDefaultTag.ToLower()
                        );
                }
                else
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{dockerCfg.UserName}/{ImageTag}".ToLower(),
                            Repo: $"{domainName}/{dockerCfg.UserName}".ToLower(),
                            FromSrc: registry.Url,
                            Auth: dockerCfg.GetRegistryAuth(registry.Url)
                        );
                }

            default:
                throw new NotSupportedException("Unsupported registry configuration");
        }
    }
}

internal sealed class PullImageHandler(
    IConnectorFactory<IImageConnector> connectorFactory, 
    IImageStreamManager imageStream, 
    IPlatformContainerCache platformContainerCache, 
    INotificationQueue notificationQueue, 
    IDbWorkQueue dbWorkQueue,
    IServiceScopeFactory scopeFactory,
    ILogger<PullImage> logger) : IStreamCommandHandler<PullImage, PullImageResult>
{
    public async IAsyncEnumerable<PullImageResult> Handle(
    PullImage command,
    [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out _))
        {
            var message = $"Platform with ID {command.PlatformId} not found.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        Registry? registry;
        if (command.RegistryName == Registry.DefaultRegistryName)
        {
            registry = Registry.DefaultRegistry();
        }
        else
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            registry = await uow.Registries.GetByNameAsync(command.RegistryName, cancellationToken);
        }

        if (registry == null)
        {
            var message = $"Registry '{command.RegistryName}' not found.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);

        await foreach (var reply in connector.PullImageProgressStreamAsync(
            command.ToConnectorCommand(platform.Address, registry), cancellationToken))
        {
            yield return reply;
        }

        var workItem = new PersistPulledImageWorkItem(
            platform,
            registry,
            command.ImageTag,
            connector,
            imageStream,
            notificationQueue,
            logger
        );

        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
    }

    internal sealed class PersistPulledImageWorkItem(
        PlatformCacheEntry platform,
        Registry registry,
        string imageName,
        IImageConnector connector,
        IImageStreamManager imageStream, 
        INotificationQueue notificationQueue,
        ILogger logger)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
        {
            // Fetch metadata
            var imageResult = await connector.GetAsync(platform.Address, imageName, ct);
            if (!imageResult.IsSuccess(out var image, out var error))
            {
                logger.LogError("Pull completed, but failed to retrieve image info: {Error}", error?.Message);
                return;
            }

            var imageEntity = image.Map(platform.Id, registry);

            // Write to DB
            await uow.Images.AddOrUpdateAsync(imageEntity, ct);
            await uow.CommitAsync(ct);

            // Notify clients
            var imageNotification = new SendImageNotificationWorkItem(imageStream, imageEntity, platform.Id);
            await notificationQueue.EnqueueAsync(imageNotification, ct);
        }
    }

    internal class SendImageNotificationWorkItem(IImageStreamManager imageStream, Image image, Guid platformId) : INotificationWorkItem
    {
        public Task ExecuteAsync(CancellationToken cancellationToken)
            => imageStream.SendImageInfo(platformId, image);
    }
}