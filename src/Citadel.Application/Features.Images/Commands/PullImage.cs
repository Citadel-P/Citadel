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
        string domainName = registry.RegistryHost.Replace("https://", "").ToLower();
        switch (registry.Configuration)
        {
            case GitHubRegistry ghCfg:
                return new PullImageCommand
                    (
                        PlatformAddress: platformAddress,
                        FromImage: $"{domainName}/{ghCfg.Name}/{BuildImageAndTag()}",
                        Auth: ghCfg.GetRegistryAuth(domainName)
                    );
            
            case CustomRegistry customCfg:
                return new PullImageCommand
                (
                    PlatformAddress: platformAddress,
                    FromImage: $"{domainName}/{BuildImageAndTag()}",
                    Auth: customCfg.AuthEnabled == true ? customCfg.GetRegistryAuth(domainName) : null
                );

            case DockerHubRegistry dockerCfg:
                if (RegistryName == Registry.DefaultRegistryName)
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: BuildImageAndTag()
                        );
                }
                else
                {
                    return new PullImageCommand
                        (
                            PlatformAddress: platformAddress,
                            FromImage: $"{domainName}/{dockerCfg.UserName}/{ImageTag}".ToLower(),
                            Auth: dockerCfg.GetRegistryAuth(domainName)
                        );
                }

            default:
                throw new NotSupportedException("Unsupported registry configuration");
        }
    }

    private string BuildImageAndTag()
        => (ImageTag.Split(':').Length == 1 ? $"{ImageTag}:latest" : ImageTag).ToLower();
}

internal sealed class PullImageHandler(
    IDbWorkQueue dbWorkQueue,
    IImageStreamManager imageStream, 
    INotificationQueue notificationQueue, 
    IPlatformContainerCache platformContainerCache, 
    IConnectorFactory<IImageConnector> connectorFactory, 
    IServiceScopeFactory scopeFactory) 
    : IStreamCommandHandler<PullImage, PullImageResult>
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

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetByNameAsync(command.RegistryName, cancellationToken);

        if (registry == null)
        {
            var message = $"Registry '{command.RegistryName}' not found.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var streamCmd = command.ToConnectorCommand(platform.Address, registry);
        await foreach (var reply in connector.PullImageProgressStreamAsync(streamCmd, cancellationToken))
        {
            yield return reply;
        }

        // Fetch metadata
        var imageResult = await connector.GetAsync(platform.Address, streamCmd.FromImage, cancellationToken);
        if (!imageResult.IsSuccess(out var image, out var error))
        {
            var message = $"Pull completed, but failed to retrieve image info: {error?.Message}.";
            yield return new PullImageResult(ErrorMessage: message, Error: new ImagePullError(404, message));
            yield break;
        }

        var imageEntity = image.Map(platform.Id, registry);

        var workItem = new PersistPulledImageWorkItem(
            imageEntity,
            platform,
            imageStream,
            notificationQueue
        );

        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
    }

    internal sealed class PersistPulledImageWorkItem(
        Image imageEntity,
        PlatformCacheEntry platform,
        IImageStreamManager imageStream, 
        INotificationQueue notificationQueue)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            // Write to DB
            await uow.Images.AddOrUpdateAsync(imageEntity, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            // Notify clients
            var imageNotification = new SendImageNotificationWorkItem(imageStream, imageEntity, platform.Id);
            await notificationQueue.EnqueueAsync(imageNotification, cancellationToken);
        }
    }

    internal class SendImageNotificationWorkItem(IImageStreamManager imageStream, Image image, Guid platformId) : INotificationWorkItem
    {
        public Task ExecuteAsync(CancellationToken cancellationToken)
            => imageStream.SendImageInfo(platformId, image);
    }
}